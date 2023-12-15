import argparse
from dateutil import parser
import json
import logging
from typing import List, Optional, Any, Dict, Union
from decimal import Decimal

import apache_beam as beam  # type: ignore
from apache_beam.options.pipeline_options import PipelineOptions  # type: ignore

from dataflow_etls.account_parsing import parse_account, OwnerProgramNotSupported, dictionify_record, \
    DispatchEventsDoFn, AccountUpdateRaw
from dataflow_etls.idl_versions import Cluster, IdlPool
from dataflow_etls.orm.accounts import AccountUpdateRecordTypes, AccountUpdateRecord


def parse_json(message: bytes) -> AccountUpdateRaw:
    account_update_raw = json.loads(message.decode("utf-8"))
    return AccountUpdateRaw(
        id=account_update_raw['id'],
        created_at=parser.parse(account_update_raw['created_at']),
        timestamp=parser.parse(account_update_raw['timestamp']),
        owner=account_update_raw['owner'],
        slot=Decimal(account_update_raw['slot']),
        pubkey=account_update_raw['pubkey'],
        txn_signature=account_update_raw['txn_signature'],
        lamports=Decimal(account_update_raw['lamports']),
        executable=bool(account_update_raw['executable']),
        rent_epoch=Decimal(account_update_raw['rent_epoch']),
        data=account_update_raw['data'],
    )


def run(
        input_topic: str,
        input_subscription: str,
        output_table_namespace: str,
        cluster: Cluster,
        min_idl_version: int,
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
    pipeline_options = PipelineOptions(beam_args, save_main_session=True, streaming=True)

    with beam.Pipeline(options=pipeline_options) as pipeline:
        # Define steps
        read_raw_txs = beam.io.ReadFromPubSub(
            topic=input_topic, subscription=input_subscription
        ).with_output_types(bytes)

        parse_to_raw_txs = beam.Map(parse_json)

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
                | "ParseTxsToRawTxs" >> parse_to_raw_txs
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
    known_args, remaining_args = parser.parse_known_args()

    run(
        input_topic=known_args.input_topic,
        input_subscription=known_args.input_subscription,
        output_table_namespace=known_args.output_table_namespace,
        cluster=known_args.cluster,
        min_idl_version=known_args.min_idl_version,
        beam_args=remaining_args,
    )


if __name__ == "__main__":
    main()
