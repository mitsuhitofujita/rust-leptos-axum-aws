# What this layer publishes. `api` reads both: the name reaches the Lambda as an
# environment variable, the ARN scopes its IAM policy. These names are an
# interface — renaming one breaks the layer that reads it, silently, until that
# layer is planned. DR-0005.

resource "aws_ssm_parameter" "table_name" {
  name  = "/${var.project}/data/table_name"
  type  = "String"
  value = aws_dynamodb_table.app.name
}

resource "aws_ssm_parameter" "table_arn" {
  name  = "/${var.project}/data/table_arn"
  type  = "String"
  value = aws_dynamodb_table.app.arn
}
