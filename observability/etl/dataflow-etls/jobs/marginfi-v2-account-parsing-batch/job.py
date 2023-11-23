import argparse
import logging
from typing import List, Optional, Any, Dict, Union
import apache_beam as beam  # type: ignore
from apache_beam.options.pipeline_options import PipelineOptions  # type: ignore

from dataflow_etls.account_parsing import parse_account, OwnerProgramNotSupported, dictionify_record, DispatchEventsDoFn
from dataflow_etls.idl_versions import Cluster, IdlPool
from dataflow_etls.orm.accounts import AccountUpdateRecordTypes, AccountUpdateRecord


def run(
        input_table: str,
        output_table_namespace: str,
        cluster: Cluster,
        min_idl_version: int,
        start_date: Optional[str] = None,
        end_date: Optional[str] = None,
        beam_args: Optional[List[str]] = None,
) -> None:
    if beam_args is None:
        beam_args = []

    idl_pool = IdlPool(cluster)

    def parse_account_internal(tx: Any) -> List[AccountUpdateRecord]:
        try:
            return parse_account(tx, min_idl_version, cluster, idl_pool)
        except OwnerProgramNotSupported:
            return []

    """Build and run the pipeline."""
    pipeline_options = PipelineOptions(beam_args, save_main_session=True)

    if start_date is not None and end_date is not None:
        input_query = f'SELECT * FROM `{input_table}` WHERE DATE(timestamp) >= "{start_date}" AND DATE(timestamp) < "{end_date}"'
    elif start_date is not None:
        input_query = (
            f'SELECT * FROM `{input_table}` WHERE DATE(timestamp) >= "{start_date}"'
        )
    elif end_date is not None:
        input_query = (
            f'SELECT * FROM `{input_table}` WHERE DATE(timestamp) < "{end_date}"'
        )
    else:
        input_query = f"SELECT * FROM `{input_table}`"

    with beam.Pipeline(options=pipeline_options) as pipeline:
        # Define steps
        read_raw_txs = beam.io.ReadFromBigQuery(query=input_query, use_standard_sql=True)

        extract_events = beam.FlatMap(parse_account_internal)

        dispatch_events = beam.ParDo(DispatchEventsDoFn()).with_outputs(
            *[rt.get_tag() for rt in AccountUpdateRecordTypes])

        dictionify_events = beam.Map(dictionify_record)

        writers: Dict[str, Union[beam.io.WriteToText, beam.io.WriteToBigQuery]] = {}
        for rt in AccountUpdateRecordTypes:
            if output_table_namespace == "local_file":  # For testing purposes
                writers[rt.get_tag()] = beam.io.WriteToText(f"account_updates_{rt.get_tag(snake_case=True)}")
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

        for rt in AccountUpdateRecordTypes:
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
        help="Output BigQuery namespace where parsed account tables are located: PROJECT:DATASET.TABLE",
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
    known_args, remaining_args = parser.parse_known_args()

    run(
        input_table=known_args.input_table,
        output_table_namespace=known_args.output_table_namespace,
        cluster=known_args.cluster,
        min_idl_version=known_args.min_idl_version,
        start_date=known_args.start_date,
        end_date=known_args.end_date,
        beam_args=remaining_args,
    )


if __name__ == "__main__":
    main()
