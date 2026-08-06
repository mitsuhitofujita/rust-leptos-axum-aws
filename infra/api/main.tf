# Everything this layer takes from the layers below it, read through SSM rather
# than through their state — DR-0005. Renaming any of these parameter names in
# the layer that writes it breaks this layer at plan time.
data "aws_ssm_parameter" "cloudfront_domain" {
  name = "/${var.project}/delivery/cloudfront_domain"
}

data "aws_ssm_parameter" "user_pool_issuer" {
  name = "/${var.project}/identity/user_pool_issuer"
}

data "aws_ssm_parameter" "app_client_id" {
  name = "/${var.project}/identity/app_client_id"
}

locals {
  name = "${var.project}-api"

  cloudfront_url   = "https://${nonsensitive(data.aws_ssm_parameter.cloudfront_domain.value)}"
  user_pool_issuer = nonsensitive(data.aws_ssm_parameter.user_pool_issuer.value)
  app_client_id    = nonsensitive(data.aws_ssm_parameter.app_client_id.value)
}
