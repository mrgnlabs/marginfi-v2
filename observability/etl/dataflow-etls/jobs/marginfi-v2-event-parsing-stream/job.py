import argparse
from dateutil import parser
import json
import logging
from typing import List, Optional, Union, Any, Dict
from decimal import Decimal

import apache_beam as beam  # type: ignore
from apache_beam.options.pipeline_options import PipelineOptions  # type: ignore

from dataflow_etls.orm.events import EventRecord, EventRecordTypes
from dataflow_etls.idl_versions import Cluster, IdlPool
from dataflow_etls.transaction_parsing import extract_events_from_tx, dictionify_record, DispatchEventsDoFn, \
    TransactionRaw, IndexedProgramNotSupported


def parse_json(message: bytes) -> TransactionRaw:
    tx_raw = json.loads(message.decode("utf-8"))
    return TransactionRaw(
        id=tx_raw["id"],
        created_at=parser.parse(tx_raw['created_at']),
        timestamp=parser.parse(tx_raw['timestamp']),
        signature=tx_raw['signature'],
        indexing_address=tx_raw['indexing_address'],
        slot=Decimal(tx_raw['slot']),
        signer=tx_raw['signer'],
        success=bool(tx_raw['success']),
        version=tx_raw['version'],
        fee=Decimal(tx_raw['fee']),
        meta=tx_raw['meta'],
        message=tx_raw['message'],
    )


def run(
        input_topic: str,
        input_subscription: str,
        output_table_namespace: str,
        cluster: Cluster,
        min_idl_version: int,
        # window_interval_sec: int = 60,
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
    pipeline_options = PipelineOptions(beam_args, save_main_session=True, streaming=True)

    with beam.Pipeline(options=pipeline_options) as pipeline:
        # Define steps
        read_raw_txs = beam.io.ReadFromPubSub(
            topic=input_topic, subscription=input_subscription
        ).with_output_types(bytes)

        parse_to_raw_txs = beam.Map(parse_json)

        # group_in_windows = beam.WindowInto(window.FixedWindows(window_interval_sec, 0))

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
                | "ParseTxsToRawTxs" >> parse_to_raw_txs
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
        "--input_topic",
        type=str,
        help='Input PubSub topic of the form "projects/<PROJECT>/topics/<TOPIC>."',
    )
    parser.add_argument(
        "--input_subscription",
        type=str,
        help='Input PubSub subscription of the form "projects/<PROJECT>/subscriptions/<SUBSCRIPTION>."',
    )
    parser.add_argument(
        "--output_table_namespace",
        type=str,
        required=True,
        help="Output BigQuery namespace where event tables are located: PROJECT:DATASET.TABLE",
    )
    parser.add_argument(
        "--window_interval_sec",
        default=60,
        type=int,
        help="Window interval in seconds for grouping incoming messages.",
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
    known_args, remaining_args = parser.parse_known_args()

    run(
        input_topic=known_args.input_topic,
        input_subscription=known_args.input_subscription,
        output_table_namespace=known_args.output_table_namespace,
        # window_interval_sec=known_args.window_interval_sec,
        cluster=known_args.cluster,
        min_idl_version=known_args.min_idl_version,
        beam_args=remaining_args,
    )


if __name__ == "__main__":
    main()
