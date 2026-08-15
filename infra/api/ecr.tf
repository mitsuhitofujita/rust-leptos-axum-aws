# The image crates/server ships as, built by infra/api/Dockerfile and pushed by
# `just deploy-api`. Terraform owns the repository; it does not own what is
# pushed into it (DR-0005), which is why there is no image resource here to
# match — the same boundary the placeholder zip drew under the form this
# replaces.
resource "aws_ecr_repository" "api" {
  name                 = local.name
  image_tag_mutability = "MUTABLE"

  image_scanning_configuration {
    scan_on_push = true
  }
}

# deploy-api always pushes the same tag, `latest`, so a push does not add a
# second tagged image — it moves the tag, and the image it moved off keeps its
# layers but loses its only tag. That is the one thing this policy prunes;
# there is no tagged-image count to cap, the same as the zip form kept no
# history of past packages either.
resource "aws_ecr_lifecycle_policy" "api" {
  repository = aws_ecr_repository.api.name

  policy = jsonencode({
    rules = [
      {
        rulePriority = 1
        description  = "Expire untagged images after 1 day"
        selection = {
          tagStatus   = "untagged"
          countType   = "sinceImagePushed"
          countUnit   = "days"
          countNumber = 1
        }
        action = { type = "expire" }
      }
    ]
  })
}

data "aws_caller_identity" "current" {}

# Lambda pulls the image using this repository policy, not the execution role —
# a role's permissions govern what the function does once invoked, not how its
# own image reaches it. The source ARN is built from local.name rather than
# read from aws_lambda_function.api.arn, so this policy has no dependency on the
# function resource: the function needs an image already in the repository to
# be created at all, so a policy that waited on the function first could never
# be satisfied on a first create.
data "aws_iam_policy_document" "ecr_lambda_pull" {
  statement {
    sid    = "AllowLambdaPull"
    effect = "Allow"

    principals {
      type        = "Service"
      identifiers = ["lambda.amazonaws.com"]
    }

    actions = [
      "ecr:BatchGetImage",
      "ecr:GetDownloadUrlForLayer",
    ]

    condition {
      test     = "StringEquals"
      variable = "aws:SourceArn"
      values   = ["arn:aws:lambda:${var.region}:${data.aws_caller_identity.current.account_id}:function:${local.name}"]
    }
  }
}

resource "aws_ecr_repository_policy" "api" {
  repository = aws_ecr_repository.api.name
  policy     = data.aws_iam_policy_document.ecr_lambda_pull.json
}
