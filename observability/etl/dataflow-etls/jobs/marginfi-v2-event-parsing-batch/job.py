import argparse
import logging
from typing import List, Optional, Union, Any, Dict
import apache_beam as beam  # type: ignore
from apache_beam.options.pipeline_options import PipelineOptions  # type: ignore

from dataflow_etls.orm.events import EventRecordTypes, EventRecord
from dataflow_etls.idl_versions import Cluster, IdlPool
from dataflow_etls.transaction_parsing import dictionify_record, DispatchEventsDoFn, extract_events_from_tx, \
    IndexedProgramNotSupported


def run(
        input_table: str,
        output_table_namespace: str,
        cluster: Cluster,
        min_idl_version: int,
        start_date: Optional[str] = None,
        end_date: Optional[str] = None,
        start_timestamp: Optional[str] = None,
        end_timestamp: Optional[str] = None,
        beam_args: Optional[List[str]] = None,
) -> None:
    if beam_args is None:
        beam_args = []

    idl_pool = IdlPool(cluster)

    def extract_events_from_tx_internal(tx: Any) -> List[EventRecord]:
        try:
            return extract_events_from_tx(tx, min_idl_version, cluster, idl_pool)
        except IndexedProgramNotSupported:
            return []

    """Build and run the pipeline."""
    pipeline_options = PipelineOptions(beam_args, save_main_session=True)

    if start_date is not None and end_date is not None:
        input_query = f'SELECT * FROM `{input_table}` WHERE DATE(timestamp) >= "{start_date}" AND DATE(timestamp) < "{end_date}"'
    elif start_timestamp is not None and end_timestamp is not None:
        input_query = f'SELECT * FROM `{input_table}` WHERE timestamp >= "{start_timestamp}" AND timestamp < "{end_timestamp}"'
    elif start_date is not None:
        input_query = (
            f'SELECT * FROM `{input_table}` WHERE DATE(timestamp) >= "{start_date}"'
        )
    elif end_date is not None:
        input_query = (
            f'SELECT * FROM `{input_table}` WHERE DATE(timestamp) < "{end_date}"'
        )
    elif end_timestamp is not None:
        input_query = (
            f'SELECT * FROM `{input_table}` WHERE timestamp < "{end_timestamp}"'
        )
        print("yes", input_query)
    else:
        input_query = f"SELECT * FROM `{input_table}`"

    with beam.Pipeline(options=pipeline_options) as pipeline:
        # Define steps
        read_raw_txs = beam.io.ReadFromBigQuery(query=input_query, use_standard_sql=True)

        extract_events = beam.FlatMap(extract_events_from_tx_internal)

        dispatch_events = beam.ParDo(DispatchEventsDoFn()).with_outputs(*[rt.get_tag() for rt in EventRecordTypes])

        dictionify_events = beam.Map(dictionify_record)

        writers: Dict[str, Union[beam.io.WriteToText, beam.io.WriteToBigQuery]] = {}
        for rt in EventRecordTypes:
            if output_table_namespace == "local_file":  # For testing purposes
                writers[rt.get_tag()] = beam.io.WriteToText(f"events_{rt.get_tag(snake_case=True)}")
            else:
                writers[rt.get_tag()] = beam.io.WriteToBigQuery(
                    f"{output_table_namespace}_{rt.get_tag(snake_case=True)}",
                    schema=rt.SCHEMA,
                    write_disposition=beam.io.BigQueryDisposition.WRITE_APPEND,
                    create_disposition=beam.io.BigQueryDisposition.CREATE_IF_NEEDED,
                )

        # Define pipeline
        tagged_events = (
                pipeline
                | "ReadRawTxs" >> read_raw_txs
                | "ExtractEvents" >> extract_events
                | "DispatchEvents" >> dispatch_events
        )

        for rt in EventRecordTypes:
            (tagged_events[rt.get_tag()]
             | f"Dictionify{rt.get_tag()}" >> dictionify_events
             | f"Write{rt.get_tag()}" >> writers[rt.get_tag()]
             )


def main() -> None:
    logging.getLogger().setLevel(logging.INFO)

    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--input_table",
        type=str,
        required=True,
        help="Input BigQuery table specified as: "
             "PROJECT.DATASET.TABLE.",
    )
    parser.add_argument(
        "--output_table_namespace",
        type=str,
        required=True,
        help="Output BigQuery namespace where event tables are located: PROJECT:DATASET.TABLE",
    )
    parser.add_argument(
        "--cluster",
        type=str,
        required=False,
        default="mainnet",
        help="Solana cluster being indexed: mainnet | devnet",
    )
    parser.add_argument(
        "--min_idl_version",
        type=int,
        required=False,
        default=0,
        help="Minimum IDL version to consider: int",
    )
    parser.add_argument(
        "--start_date",
        type=str,
        help="Start date to consider (inclusive) as: YYYY-MM-DD",
    )
    parser.add_argument(
        "--end_date",
        type=str,
        help="End date to consider (exclusive) as: YYYY-MM-DD",
    )
    parser.add_argument(
        "--start_timestamp",
        type=str,
        help="Start timestamp to consider (inclusive) as: YYYY-MM-DD HH:MM:SS",
    )
    parser.add_argument(
        "--end_timestamp",
        type=str,
        help="End timestamp to consider (exclusive) as: YYYY-MM-DD HH:MM:SS",
    )
    known_args, remaining_args = parser.parse_known_args()

    run(
        input_table=known_args.input_table,
        output_table_namespace=known_args.output_table_namespace,
        cluster=known_args.cluster,
        min_idl_version=known_args.min_idl_version,
        start_date=known_args.start_date,
        end_date=known_args.end_date,
        start_timestamp=known_args.start_timestamp,
        end_timestamp=known_args.end_timestamp,
        beam_args=remaining_args,
    )


if __name__ == "__main__":
    main()
