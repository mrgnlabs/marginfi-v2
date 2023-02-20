- Test pipeline locally:

```
python job.py \
    --temp_location gs://dataflow_jobs_marginfi_v2/tmp/ \
    --project marginfi-dev \
    --input_table marginfi-dev.marginfi_v2_devnet.transactions_raw \
    --output_table local_file \
    --start_date 2022-11-27 \
    --end_date 2022-11-29
```

- Build image and upload to Artifact Registry:

```
gcloud builds submit --tag us-east1-docker.pkg.dev/marginfi-dev/main/dataflow/event-parsing-etl-batch:latest .
```

- Create/Update template and associate metadata file:

```
gcloud dataflow flex-template build \
    gs://dataflow_jobs_marginfi_v2/templates/event-parsing-etl-batch.json \
    --image "us-east1-docker.pkg.dev/marginfi-dev/main/dataflow/event-parsing-etl-batch:latest" \
    --sdk-language "PYTHON" \
    --metadata-file "metadata.json"
```
