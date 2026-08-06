# What this layer publishes. `identity` and `api` read cloudfront_domain; the SPA
# build and the deploy commands read the rest. These names are an interface —
# renaming one breaks the layer that reads it, silently, until that layer is
# planned. DR-0005.

resource "aws_ssm_parameter" "cloudfront_domain" {
  name  = "/${var.project}/delivery/cloudfront_domain"
  type  = "String"
  value = aws_cloudfront_distribution.spa.domain_name
}

resource "aws_ssm_parameter" "cloudfront_distribution_id" {
  name  = "/${var.project}/delivery/cloudfront_distribution_id"
  type  = "String"
  value = aws_cloudfront_distribution.spa.id
}

resource "aws_ssm_parameter" "spa_bucket" {
  name  = "/${var.project}/delivery/spa_bucket"
  type  = "String"
  value = aws_s3_bucket.spa.id
}
