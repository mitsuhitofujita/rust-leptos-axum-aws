# bucket, region, encrypt and use_lockfile come from infra/backend.hcl, which is
# not committed. See infra/backend.hcl.example.
terraform {
  backend "s3" {
    key = "api/terraform.tfstate"
  }
}
