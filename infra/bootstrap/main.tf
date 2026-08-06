# The account id is read rather than written down: S3 bucket names are globally
# unique and an account id is the conventional suffix, but the id itself is kept
# out of the repository.
data "aws_caller_identity" "current" {}

locals {
  state_bucket = "${var.project}-tfstate-${data.aws_caller_identity.current.account_id}"
}

resource "aws_s3_bucket" "state" {
  bucket = local.state_bucket

  # Destroying this bucket loses the record of every other layer. prevent_destroy
  # stops Terraform from doing it; nothing stops the console, which is what the
  # versioning below is for — DR-0005.
  lifecycle {
    prevent_destroy = true
  }
}

resource "aws_s3_bucket_versioning" "state" {
  bucket = aws_s3_bucket.state.id

  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_server_side_encryption_configuration" "state" {
  bucket = aws_s3_bucket.state.id

  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "AES256"
    }
  }
}

resource "aws_s3_bucket_public_access_block" "state" {
  bucket = aws_s3_bucket.state.id

  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

resource "aws_s3_bucket_ownership_controls" "state" {
  bucket = aws_s3_bucket.state.id

  rule {
    object_ownership = "BucketOwnerEnforced"
  }
}

# Published so the bucket name can be found without reading anyone's state, in
# the same way every other cross-layer value travels — DR-0005.
resource "aws_ssm_parameter" "state_bucket" {
  name  = "/${var.project}/bootstrap/state_bucket"
  type  = "String"
  value = aws_s3_bucket.state.id
}
