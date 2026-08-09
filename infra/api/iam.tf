data "aws_iam_policy_document" "lambda_assume_role" {
  statement {
    actions = ["sts:AssumeRole"]

    principals {
      type        = "Service"
      identifiers = ["lambda.amazonaws.com"]
    }
  }
}

resource "aws_iam_role" "lambda" {
  name               = "${local.name}-lambda"
  assume_role_policy = data.aws_iam_policy_document.lambda_assume_role.json
}

resource "aws_iam_role_policy_attachment" "lambda_basic" {
  role       = aws_iam_role.lambda.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole"
}

# The item operations the service issues against the one table the data layer
# owns, and nothing else. Scan is deliberately absent: every access pattern in
# docs/design/persistence.md is a Query inside one user's partition, so a Scan
# reaching this table would be a bug, and leaving the permission out is what
# turns that bug into an error rather than a full-table read.
#
# The resource is the table alone — there is no index to grant, because the
# table has none. One user's items are not separated from another's here either:
# the function serves every user and derives the partition key from the verified
# token, so the isolation is in the service, not in this policy.
data "aws_iam_policy_document" "lambda_table" {
  statement {
    actions = [
      "dynamodb:GetItem",
      "dynamodb:BatchGetItem",
      "dynamodb:Query",
      "dynamodb:PutItem",
      "dynamodb:UpdateItem",
      "dynamodb:DeleteItem",
      "dynamodb:BatchWriteItem",
      "dynamodb:TransactWriteItems",
    ]
    resources = [local.table_arn]
  }
}

resource "aws_iam_role_policy" "lambda_table" {
  name   = "${local.name}-table"
  role   = aws_iam_role.lambda.id
  policy = data.aws_iam_policy_document.lambda_table.json
}
