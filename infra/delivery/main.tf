data "aws_caller_identity" "current" {}

locals {
  spa_bucket = "${var.project}-spa-${data.aws_caller_identity.current.account_id}"
}

# ---------------------------------------------------------------------------
# The origin bucket
#
# Private, always. Nothing reaches it but CloudFront through the Origin Access
# Control below. Terraform owns the bucket; `aws s3 sync` owns its contents —
# DR-0005.
# ---------------------------------------------------------------------------

resource "aws_s3_bucket" "spa" {
  bucket = local.spa_bucket
}

resource "aws_s3_bucket_versioning" "spa" {
  bucket = aws_s3_bucket.spa.id

  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_server_side_encryption_configuration" "spa" {
  bucket = aws_s3_bucket.spa.id

  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "AES256"
    }
  }
}

resource "aws_s3_bucket_public_access_block" "spa" {
  bucket = aws_s3_bucket.spa.id

  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

resource "aws_s3_bucket_ownership_controls" "spa" {
  bucket = aws_s3_bucket.spa.id

  rule {
    object_ownership = "BucketOwnerEnforced"
  }
}

data "aws_iam_policy_document" "spa" {
  statement {
    sid       = "AllowCloudFrontRead"
    actions   = ["s3:GetObject"]
    resources = ["${aws_s3_bucket.spa.arn}/*"]

    principals {
      type        = "Service"
      identifiers = ["cloudfront.amazonaws.com"]
    }

    # Scoped to this distribution rather than to the service as a whole, so no
    # other account's distribution can read the bucket.
    condition {
      test     = "StringEquals"
      variable = "AWS:SourceArn"
      values   = [aws_cloudfront_distribution.spa.arn]
    }
  }
}

resource "aws_s3_bucket_policy" "spa" {
  bucket = aws_s3_bucket.spa.id
  policy = data.aws_iam_policy_document.spa.json
}

# ---------------------------------------------------------------------------
# The distribution
# ---------------------------------------------------------------------------

resource "aws_cloudfront_origin_access_control" "spa" {
  name                              = "${var.project}-spa"
  description                       = "Signs CloudFront's requests to the private SPA bucket."
  origin_access_control_origin_type = "s3"
  signing_behavior                  = "always"
  signing_protocol                  = "sigv4"
}

# AWS's managed cache policies, resolved by name rather than by their well-known
# ids so the configuration says what it means.
data "aws_cloudfront_cache_policy" "caching_optimized" {
  name = "Managed-CachingOptimized"
}

data "aws_cloudfront_cache_policy" "caching_disabled" {
  name = "Managed-CachingDisabled"
}

resource "aws_cloudfront_distribution" "spa" {
  enabled             = true
  is_ipv6_enabled     = true
  comment             = "${var.project} SPA"
  default_root_object = "index.html"
  price_class         = "PriceClass_200"

  origin {
    origin_id                = "spa"
    domain_name              = aws_s3_bucket.spa.bucket_regional_domain_name
    origin_access_control_id = aws_cloudfront_origin_access_control.spa.id
  }

  # Everything but the entry point is content-hashed by trunk, so it can be
  # cached hard: a new build produces new filenames.
  default_cache_behavior {
    target_origin_id       = "spa"
    viewer_protocol_policy = "redirect-to-https"
    allowed_methods        = ["GET", "HEAD"]
    cached_methods         = ["GET", "HEAD"]
    compress               = true
    cache_policy_id        = data.aws_cloudfront_cache_policy.caching_optimized.id
  }

  # index.html is the one file trunk does not hash. Caching it would serve a
  # stale bundle reference to every returning visitor.
  ordered_cache_behavior {
    path_pattern           = "/index.html"
    target_origin_id       = "spa"
    viewer_protocol_policy = "redirect-to-https"
    allowed_methods        = ["GET", "HEAD"]
    cached_methods         = ["GET", "HEAD"]
    compress               = true
    cache_policy_id        = data.aws_cloudfront_cache_policy.caching_disabled.id
  }

  # Deep links are router paths, not files, so an unknown key has to return the
  # SPA with a 200 — DR-0001. 403 matters as much as 404: the bucket is private
  # behind OAC, so a missing key comes back as AccessDenied, not NoSuchKey.
  custom_error_response {
    error_code            = 403
    response_code         = 200
    response_page_path    = "/index.html"
    error_caching_min_ttl = 0
  }

  custom_error_response {
    error_code            = 404
    response_code         = 200
    response_page_path    = "/index.html"
    error_caching_min_ttl = 0
  }

  restrictions {
    geo_restriction {
      restriction_type = "none"
    }
  }

  # The default *.cloudfront.net certificate. A custom domain would need an ACM
  # certificate in us-east-1 and would make this the only cross-region resource
  # in the stack.
  viewer_certificate {
    cloudfront_default_certificate = true
  }
}
