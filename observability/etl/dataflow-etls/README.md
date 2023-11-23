- Test pipeline locally:

```
python job.py \
    --temp_location gs://dataflow_jobs_marginfi_v2/tmp/ \
    --project marginfi-dev \
    --input_table marginfi-dev.marginfi_v2_devnet.transactions_raw \
    --output_table_namespace local_file \
    --cluster devnet \
    --min_idl_version 3 \
    --start_date 2022-11-27 \
    --end_date 2022-11-29
```

- Build image and upload to Artifact Registry:

```
./scripts/build_job_template <PATH TO JOB DIR> <VERSION>
./scripts/upload_job_template <PATH TO JOB DIR> <VERSION>
```

- Create/Update template and associate metadata file:

```
./scripts/sync_job_template <PATH TO JOB DIR> <VERSION>
```
