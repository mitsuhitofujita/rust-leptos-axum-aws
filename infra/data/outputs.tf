output "table_name" {
  description = "The single application table."
  value       = aws_dynamodb_table.app.name
}

output "table_arn" {
  description = "What the api layer scopes the Lambda's item permissions to."
  value       = aws_dynamodb_table.app.arn
}
