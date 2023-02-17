import argparse
import base64
import json
import logging
from typing import List, Optional
from anchorpy import Event
from anchorpy.coder.event import EVENT_DISCRIMINATOR_SIZE
from solders.message import MessageV0, Message
import apache_beam as beam
from apache_beam.options.pipeline_options import PipelineOptions
from solders.pubkey import Pubkey

from event_parsing_etl_batch.event_records import event_to_record, Record, LiquidityChangeRecord, \
    MarginfiAccountCreationRecord
from event_parsing_etl_batch.transaction_log_parser import reconcile_instruction_logs, \
    merge_instructions_and_cpis, expand_instructions, InstructionWithLogs, PROGRAM_DATA
from event_parsing_etl_batch.client_generated.events import *

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


def process_event(record_list: List[Record], event: Event) -> None:
    maybe_record = event_to_record(event)
    if maybe_record is not None:
        record_list.append(maybe_record)


class DispatchEventsDoFn(beam.DoFn):
    def process(self, record: Record, *args, **kwargs):
        yield beam.pvalue.TaggedOutput(record.NAME, record)


def create_records_from_ix(ix: InstructionWithLogs) -> List[Record]:
    records = []
    for log in ix.logs:
        if log.startswith(PROGRAM_DATA):
            event_encoded = log[len(PROGRAM_DATA):]
            event_bytes = base64.b64decode(event_encoded)

            disc = event_bytes[:EVENT_DISCRIMINATOR_SIZE]
            if disc == MarginfiAccountCreateEvent.discriminator:
                event = Event(name="MarginfiAccountCreateEvent", data=MarginfiAccountCreateEvent.decode(event_bytes))
            elif disc == LendingAccountDepositEvent.discriminator:
                event = Event(name="LendingAccountDepositEvent", data=LendingAccountDepositEvent.decode(event_bytes))
            elif disc == LendingAccountWithdrawEvent.discriminator:
                event = Event(name="LendingAccountWithdrawEvent", data=LendingAccountWithdrawEvent.decode(event_bytes))
            elif disc == LendingAccountBorrowEvent.discriminator:
                event = Event(name="LendingAccountBorrowEvent", data=LendingAccountBorrowEvent.decode(event_bytes))
            elif disc == LendingAccountRepayEvent.discriminator:
                event = Event(name="LendingAccountRepayEvent", data=LendingAccountRepayEvent.decode(event_bytes))
            elif disc == LendingAccountWithdrawEvent.discriminator:
                event = Event(name="LendingAccountWithdrawEvent", data=LendingAccountWithdrawEvent.decode(event_bytes))
            elif disc == LendingPoolBankAddEvent.discriminator:
                event = Event(name="LendingPoolBankAddEvent", data=LendingPoolBankAddEvent.decode(event_bytes))
            elif disc == LendingPoolBankAccrueInterestEvent.discriminator:
                event = Event(name="LendingPoolBankAccrueInterestEvent",
                              data=LendingPoolBankAccrueInterestEvent.decode(event_bytes))
            else:
                print("Program event not supported", event_bytes, log)
                event = None

            # todo: decode ix and enrich when needed ^
            if event is not None:
                print(event.name)

    return records


def extract_events_from_ix(ix: InstructionWithLogs, program_id: Pubkey) -> List[Record]:
    ix_events = []

    if ix.message.program_id == program_id:
        ix_events.extend(create_records_from_ix(ix))

    for inner_ix in ix.inner_instructions:
        ix_events.extend(extract_events_from_ix(inner_ix, program_id))

    return ix_events


def run(
        input_table: str,
        target_dataset: str,
        start_date: Optional[str] = None,
        end_date: Optional[str] = None,
        beam_args: List[str] = None,
) -> None:
    # path = Path("marginfi.json")
    # raw = path.read_text()

    def extract_events_from_tx(tx):
        # idl = Idl.from_json(raw)
        indexed_program_id = Pubkey.from_string(tx["indexing_address"])
        # program = Program(idl, Pubkey.from_string(indexed_program))

        meta = json.loads(tx["meta"])
        message_bytes = base64.b64decode(tx["message"])

        tx_version = tx["version"]
        if tx_version == "legacy":
            message_decoded = Message.from_bytes(message_bytes)
        elif tx_version == "0":
            message_decoded = MessageV0.from_bytes(message_bytes[1:])
        else:
            return []

        merged_instructions = merge_instructions_and_cpis(message_decoded.instructions, meta["innerInstructions"])
        expanded_instructions = expand_instructions(message_decoded.account_keys, merged_instructions)
        ixs_with_logs = reconcile_instruction_logs(expanded_instructions, meta["logMessages"])

        records_list = []
        for ix_with_logs in ixs_with_logs:
            records_list.extend(extract_events_from_ix(ix_with_logs, indexed_program_id))

        return records_list

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
            LiquidityChangeRecord.NAME,
            MarginfiAccountCreationRecord.NAME,
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

        tagged_events[LiquidityChangeRecord.NAME] | "WriteLiquidityChangeEvent" >> write_liquidity_change_events
        tagged_events[
            MarginfiAccountCreationRecord.NAME] | "WriteMarginfiAccountCreationEvent" >> write_marginfi_account_creation_events


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
