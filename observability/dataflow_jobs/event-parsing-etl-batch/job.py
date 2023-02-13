import argparse
import logging
import json
import types
import uuid
from typing import Any, Dict, List, Optional, TypedDict
from pathlib import Path
from anchorpy import Idl, EventParser, Program, EventParser
from solders.pubkey import Pubkey

import apache_beam as beam
from apache_beam.options.pipeline_options import PipelineOptions

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


def extract_events(tx: Dict[str, Any]) -> List[object]:
    path = Path("marginfi.json")
    raw = path.read_text()
    idl = Idl.from_json(raw)

    indexed_program = tx["indexing_address"]
    
    program = Program(idl, Pubkey.from_string(indexed_program))
    event_parser = EventParser(program.program_id, program.coder)

    meta = json.loads(tx["meta"], object_hook=lambda d: types.SimpleNamespace(**d))

    events = []
    event_parser.parse_logs(meta.logMessages, lambda evt: events.append(evt))
    print(events)

    return [{
        "id": str(uuid.uuid4()),
        "timestamp": tx["timestamp"],
        "signature": tx["signature"],
        "signer": tx["signer"],
        "indexing_address": tx["indexing_address"],
        "fee": tx["fee"],
    }]


def run(
        input_table: str,
        output_table: str,
        start_date: Optional[str] = None,
        end_date: Optional[str] = None,
        beam_args: List[str] = None,
) -> None:
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
        read = beam.io.ReadFromBigQuery(query=input_query, use_standard_sql=True)
        transform = beam.FlatMap(extract_events)
        if output_table == "local_file":
            write = beam.io.WriteToText("local_file")  # For testing purposes
        else:
            write = beam.io.WriteToBigQuery(
                output_table,
                schema=PROCESSED_TRANSACTION_SCHEMA,
                write_disposition=beam.io.BigQueryDisposition.WRITE_APPEND,
            )

        # Define pipeline
        (
                pipeline
                | "ReadRawTxs" >> read
                | "TransformTxs" >> transform
                | "WriteProcessedTxs" >> write
        )


if __name__ == "__main__":
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
        "--output_table",
        type=str,
        required=True,
        help="Output BigQuery table for results specified as: "
             "PROJECT:DATASET.TABLE or DATASET.TABLE.",
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
    args, remaining_args = parser.parse_known_args()

    run(
        input_table=args.input_table,
        output_table=args.output_table,
        start_date=args.start_date,
        end_date=args.end_date,
        beam_args=remaining_args,
    )
