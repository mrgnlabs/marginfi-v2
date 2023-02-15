import argparse
import logging
import json
import types
import uuid
from dataclasses import dataclass
from typing import Any, Dict, List, Optional, TypedDict
from pathlib import Path
from anchorpy import Idl, Program
from attr import dataclass
from solders.pubkey import Pubkey
import apache_beam as beam
from apache_beam.options.pipeline_options import PipelineOptions

from event_parsing_etl_batch.event_parser import EventParser, Event

# Defines the BigQuery schema for the output table.
PROCESSED_TRANSACTION_SCHEMA = ",".join(
    [
        "id:STRING",
        "timestamp:TIMESTAMP",
        "signature:STRING",
        "signer:STRING",
        "indexing_address:STRING",
        "fee:BIGNUMERIC",
    ]
)


@dataclass
class LiquidityChangeRecord:
    version: str
    marginfi_group: Pubkey
    marginfi_account: Pubkey
    authority: Pubkey
    operation: str
    amount: int
    balance_closed: bool


@dataclass
class MarginfiAccountCreationRecord:
    version: str
    marginfi_group: Pubkey
    marginfi_account: Pubkey
    authority: Pubkey


class DispatchEventsDoFn(beam.DoFn):
    # Event names
    LENDING_ACCOUNT_DEPOSIT_EVENT = 'LendingAccountDepositEvent'
    LENDING_ACCOUNT_WITHDRAW_EVENT = 'LendingAccountWithdrawEvent'
    LENDING_ACCOUNT_BORROW_EVENT = 'LendingAccountBorrowEvent'
    LENDING_ACCOUNT_REPAY_EVENT = 'LendingAccountRepayEvent'
    MARGINFI_ACCOUNT_CREATE_EVENT = 'MarginfiAccountCreateEvent'

    # Event category
    LIQUIDITY_CHANGE_CATEGORY = 'LIQUIDITY_CHANGE_CATEGORY'
    MARGINFI_ACCOUNT_CREATION_CATEGORY = 'MARGINFI_ACCOUNT_CREATION_CATEGORY'
    LENDING_POOL_BANK_ACCRUE_INTEREST_CATEGORY = 'LENDING_POOL_BANK_ACCRUE_INTEREST_CATEGORY'
    LENDING_POOL_BANK_ADD_CATEGORY = 'LENDING_POOL_BANK_ADD_CATEGORY'

    @staticmethod
    def is_liquidity_change_event(event_name: str):
        return event_name in [
            DispatchEventsDoFn.LENDING_ACCOUNT_DEPOSIT_EVENT,
            DispatchEventsDoFn.LENDING_ACCOUNT_WITHDRAW_EVENT,
            DispatchEventsDoFn.LENDING_ACCOUNT_BORROW_EVENT,
            DispatchEventsDoFn.LENDING_ACCOUNT_REPAY_EVENT,
        ]

    @staticmethod
    def shape_liquidity_change_event(event: Event) -> LiquidityChangeRecord:
        balance_closed = None
        if event.name == DispatchEventsDoFn.LENDING_ACCOUNT_REPAY_EVENT or event.name == DispatchEventsDoFn.LENDING_ACCOUNT_WITHDRAW_EVENT:
            balance_closed = event.data.close_balance

        return LiquidityChangeRecord(operation=event.name,
                                     version=event.data.header.version,
                                     marginfi_account=event.data.header.marginfi_account,
                                     marginfi_group=event.data.header.marginfi_group,
                                     authority=event.data.header.signer,
                                     amount=event.data.amount,
                                     balance_closed=balance_closed)

    @staticmethod
    def shape_marginfi_account_creation_event(event: Event) -> MarginfiAccountCreationRecord:
        return MarginfiAccountCreationRecord(version=event.data.header.version,
                                             marginfi_account=event.data.header.marginfi_account,
                                             marginfi_group=event.data.header.marginfi_group,
                                             authority=event.data.header.signer)

    def process(self, event, *args, **kwargs):
        if self.is_liquidity_change_event(event.name):
            processed_event = self.shape_liquidity_change_event(event)
            yield beam.pvalue.TaggedOutput(self.LIQUIDITY_CHANGE_CATEGORY, processed_event)
        elif event.name == self.MARGINFI_ACCOUNT_CREATE_EVENT:
            processed_event = self.shape_marginfi_account_creation_event(event)
            yield beam.pvalue.TaggedOutput(self.MARGINFI_ACCOUNT_CREATION_CATEGORY, processed_event)
        else:
            print("discarding unsupported event:", event.name)


def run(
        input_table: str,
        target_dataset: str,
        start_date: Optional[str] = None,
        end_date: Optional[str] = None,
        beam_args: List[str] = None,
) -> None:
    path = Path("marginfi.json")
    raw = path.read_text()

    def extract_events_from_tx(tx):
        idl = Idl.from_json(raw)
        indexed_program = tx["indexing_address"]
        program = Program(idl, Pubkey.from_string(indexed_program))
        event_parser = EventParser(program.program_id, program.coder)
        meta = json.loads(tx["meta"], object_hook=lambda d: types.SimpleNamespace(**d))

        events = []
        event_parser.parse_logs(meta.logMessages, lambda event: events.append(event))
        return events

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

        extract_events = beam.FlatMap(extract_events_from_tx)

        dispatch_events = beam.ParDo(DispatchEventsDoFn()).with_outputs(
            DispatchEventsDoFn.LIQUIDITY_CHANGE_CATEGORY,
            DispatchEventsDoFn.MARGINFI_ACCOUNT_CREATION_CATEGORY
        )

        if target_dataset == "local_file":  # For testing purposes
            write_liquidity_change_events = beam.io.WriteToText("local_file_liquidity_change_events")
            write_marginfi_account_creation_events = beam.io.WriteToText("local_file_marginfi_account_creation_events")
        else:
            print("TODOOOOO")
            exit(1)
            # write_liquidity_change_events = beam.io.WriteToBigQuery(
            #     output_table,
            #     schema=PROCESSED_TRANSACTION_SCHEMA,
            #     write_disposition=beam.io.BigQueryDisposition.WRITE_APPEND,
            # )

        # Define pipeline
        tagged_events = (
                pipeline
                | "ReadRawTxs" >> read_raw_txs
                | "ExtractEvents" >> extract_events
                | "DispatchEvents" >> dispatch_events
        )

        tagged_events.LIQUIDITY_CHANGE_CATEGORY | "WriteLiquidityChangeEvent" >> write_liquidity_change_events
        tagged_events.MARGINFI_ACCOUNT_CREATION_CATEGORY | "WriteMarginfiAccountCreationEvent" >> write_marginfi_account_creation_events


def main():
    logging.getLogger().setLevel(logging.INFO)

    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--input_table",
        type=str,
        required=True,
        help="Input BigQuery table specified as: "
             "PROJECT:DATASET.TABLE or DATASET.TABLE.",
    )
    parser.add_argument(
        "--target_dataset",
        type=str,
        required=True,
        help="Output BigQuery dataset where event tables are located: PROJECT:DATASET",
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
        target_dataset=known_args.target_dataset,
        start_date=known_args.start_date,
        end_date=known_args.end_date,
        beam_args=remaining_args,
    )


if __name__ == "__main__":
    main()
