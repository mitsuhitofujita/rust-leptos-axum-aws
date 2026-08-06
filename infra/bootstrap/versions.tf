# No backend block: bootstrap creates the bucket every other layer stores its
# state in, so its first apply has nowhere remote to go and runs against a local
# state file. Once the bucket exists, copy backend.tf.example to backend.tf and
# run `terraform init -migrate-state -backend-config=../backend.hcl` — DR-0005.
terraform {
  required_version = ">= 1.11.0"

  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 6.0"
    }
  }
}

provider "aws" {
  region = var.region

  default_tags {
    tags = {
      Project = var.project
      Layer   = "bootstrap"
    }
  }
}
