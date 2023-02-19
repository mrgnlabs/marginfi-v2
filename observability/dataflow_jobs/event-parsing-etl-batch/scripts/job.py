import argparse
import base64
import json
import logging
from typing import List, Optional
from solders.message import MessageV0, Message
import apache_beam as beam
from apache_beam.options.pipeline_options import PipelineOptions
from solders.pubkey import Pubkey

from event_parsing_etl_batch.orm import Record, LiquidityChangeRecord, \
    MarginfiAccountCreationRecord, is_liquidity_change_event, MARGINFI_ACCOUNT_CREATE_EVENT, LendingPoolBankAddRecord, \
    LendingPoolBankAccrueInterestRecord, LENDING_POOL_BANK_ACCRUE_INTEREST_EVENT, LENDING_POOL_BANK_ADD_EVENT
from event_parsing_etl_batch.idl_versions import VersionedIdl, VersionedProgram, Cluster
from event_parsing_etl_batch.transaction_log_parser import reconcile_instruction_logs, \
    merge_instructions_and_cpis, expand_instructions, InstructionWithLogs, PROGRAM_DATA


class DispatchEventsDoFn(beam.DoFn):
    def process(self, record: Record, *args, **kwargs):
        yield beam.pvalue.TaggedOutput(record.NAME, record)


def create_records_from_ix(ix: InstructionWithLogs, program: VersionedProgram) -> List[Record]:
    records = []
    for log in ix.logs:
        if not log.startswith(PROGRAM_DATA):
            continue

        event_encoded = log[len(PROGRAM_DATA):]
        try:
            event_bytes = base64.b64decode(event_encoded)
        except Exception as e:
            print(f"error: failed to decode base64 event string in tx {ix.signature}", e)
            continue

        print(f"info decoded with IDL {program.version}")

        try:
            event = program.coder.events.parse(event_bytes)
        except Exception as e:
            print(f"failed to parse event in tx {ix.signature}", e)
            continue

        try:
            instruction_data = program.coder.instruction.parse(ix.message.data)
        except Exception as e:
            print(ix)
            print(f"failed to parse instruction data in tx {ix.signature}", e)
            continue

        if is_liquidity_change_event(event.name):
            record = LiquidityChangeRecord.from_event(event)
        elif event.name == MARGINFI_ACCOUNT_CREATE_EVENT:
            record = MarginfiAccountCreationRecord.from_event(event)
        elif event.name == LENDING_POOL_BANK_ADD_EVENT:
            record = LendingPoolBankAddRecord.from_event(event)
        elif event.name == LENDING_POOL_BANK_ACCRUE_INTEREST_EVENT:
            record = LendingPoolBankAccrueInterestRecord.from_event(event, ix, instruction_data)
        else:
            print("discarding unsupported event:", event.name)
            record = None

        if record is not None:
            records.append(record)

    return records


def extract_events_from_ix(ix: InstructionWithLogs, program: VersionedProgram) -> List[Record]:
    ix_events = []

    if ix.message.program_id == program.program_id:
        ix_events.extend(create_records_from_ix(ix, program))

    for inner_ix in ix.inner_instructions:
        ix_events.extend(extract_events_from_ix(inner_ix, program))

    return ix_events


def run(
        input_table: str,
        target_dataset: str,
        cluster: Cluster,
        start_date: Optional[str] = None,
        end_date: Optional[str] = None,
        beam_args: List[str] = None,
) -> None:
    def extract_events_from_tx(tx):
        indexed_program_id_str = tx["indexing_address"]
        indexed_program_id = Pubkey.from_string(indexed_program_id_str)
        tx_slot = int(tx["slot"])
        idl, idl_version = VersionedIdl.get_idl_for_slot(cluster, indexed_program_id_str, tx_slot)
        program = VersionedProgram(cluster, idl_version, idl, indexed_program_id)

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
        ixs_with_logs = reconcile_instruction_logs(tx["signature"], expanded_instructions, meta["logMessages"])

        records_list = []
        for ix_with_logs in ixs_with_logs:
            records_list.extend(extract_events_from_ix(ix_with_logs, program))

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
            LendingPoolBankAddRecord.NAME,
            LendingPoolBankAccrueInterestRecord.NAME,
        )

        if target_dataset == "local_file":  # For testing purposes
            write_liquidity_change_events = beam.io.WriteToText("local_file_liquidity_change_events")
            write_marginfi_account_creation_events = beam.io.WriteToText("local_file_marginfi_account_creation_events")
            write_lending_pool_bank_add_events = beam.io.WriteToText("local_file_lending_pool_bank_add_events")
            write_lending_pool_bank_accrue_interest_events = beam.io.WriteToText(
                "local_file_lending_pool_bank_accrue_interest_events")
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
        tagged_events[
            LendingPoolBankAddRecord.NAME] | "WriteLendingPoolBankAddEvent" >> write_lending_pool_bank_add_events
        tagged_events[
            LendingPoolBankAccrueInterestRecord.NAME] | "WriteLendingPoolBankAccrueInterestEvent" >> write_lending_pool_bank_accrue_interest_events


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
        "--cluster",
        type=str,
        required=False,
        default="mainnet",
        help="Solana cluster being indexed: mainnet | devnet",
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
        cluster=known_args.cluster,
        start_date=known_args.start_date,
        end_date=known_args.end_date,
        beam_args=remaining_args,
    )


if __name__ == "__main__":
    main()
