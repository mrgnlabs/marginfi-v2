[View code on GitHub](https://github.com/mrgnlabs/marginfi-v2/observability/etl/dataflow-etls/metadata.json)

This code defines a JSON object that describes the parameters for a batch job that parses individual raw transactions from a BigQuery table and stores them in another BigQuery table. The `name` field specifies the name of the batch job, which is "event-parsing-batch". The `description` field provides a brief description of what the batch job does.

The `parameters` field is an array of objects that define the input parameters for the batch job. Each object has several fields that provide information about the parameter, such as its name, label, help text, and regular expression for validation. The input parameters for this batch job are:

- `input_table`: The name of the input table to consume from.
- `output_table_namespace`: The namespace where the BigQuery output tables are located.
- `cluster`: The Solana cluster where the processed transactions are executed. This parameter is optional and has a default value of "mainnet".
- `min_idl_version`: The minimum IDL version for which transactions will be parsed. This parameter is optional and has a default value of 0.
- `start_date`: The start date to consider (inclusive). This parameter is optional.
- `end_date`: The end date to consider (exclusive). This parameter is optional.

This code is likely used in the larger marginfi-v2 project to define the input parameters for the event parsing batch job. These parameters can be used to configure the batch job and customize its behavior based on the specific needs of the project. For example, the `cluster` parameter can be used to specify which Solana cluster to use for processing transactions, while the `min_idl_version` parameter can be used to filter out transactions that do not meet a certain IDL version requirement. Overall, this code provides a flexible and customizable way to configure the event parsing batch job for the marginfi-v2 project.
## Questions: 
 1. What is the purpose of this code and how does it work?
- This code is for parsing individual raw transactions from a BigQuery table and storing them in another BigQuery table. It takes in parameters such as the input table name, output table namespace, Solana cluster, minimum IDL version, start and end dates to consider.

2. What are the expected formats for the input and output table names?
- The input table name should be in the format of "([^.]+.)?[^.]+[.].+" and the output table namespace should be in the format of "([^:]+:)?[^.]+[.].+".

3. What are the valid options for the "cluster" parameter?
- The valid options for the "cluster" parameter are "mainnet" and "devnet".