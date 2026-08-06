output "state_bucket" {
  description = "The bucket every layer's state file lives in."
  value       = aws_s3_bucket.state.id
}

# infra/backend.hcl is not committed, because it carries the account id. This
# output is its exact contents, so the operator copies rather than derives it:
#   terraform -chdir=infra/bootstrap output -raw backend_hcl > infra/backend.hcl
output "backend_hcl" {
  description = "Contents for infra/backend.hcl."
  value       = <<-EOT
    bucket       = "${aws_s3_bucket.state.id}"
    region       = "${var.region}"
    encrypt      = true
    use_lockfile = true
  EOT
}
