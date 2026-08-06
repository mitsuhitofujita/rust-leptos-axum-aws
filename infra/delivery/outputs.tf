output "cloudfront_domain" {
  description = "The domain the SPA is served from."
  value       = aws_cloudfront_distribution.spa.domain_name
}

output "cloudfront_distribution_id" {
  description = "Target of `aws cloudfront create-invalidation` after a deploy."
  value       = aws_cloudfront_distribution.spa.id
}

output "spa_bucket" {
  description = "Target of `aws s3 sync dist/` after a build."
  value       = aws_s3_bucket.spa.id
}
