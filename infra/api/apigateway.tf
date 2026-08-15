locals {
  # The methods the SPA calls. The CORS configuration and the routes below both
  # derive from this list, because the two have to agree: a method the routes
  # accept and CORS does not is blocked in the browser, and the reverse is a 404
  # the preflight does not predict.
  api_methods = ["GET", "POST"]
}

resource "aws_apigatewayv2_api" "this" {
  name          = local.name
  protocol_type = "HTTP"

  # DR-0001 records the missing CORS layer in crates/server as a gap deployment
  # has to close; it is closed here rather than in the service. An HTTP API
  # answers preflight itself, ahead of any authorizer, but only for an OPTIONS
  # request no route matches — hence the enumerated routes below.
  cors_configuration {
    allow_origins = [local.cloudfront_url]
    allow_methods = concat(local.api_methods, ["OPTIONS"])
    allow_headers = ["authorization", "content-type"]
    max_age       = 3600
  }
}

# Two integrations to one function, differing only in the headers they map.
#
# crates/server reads an AuthContext of its own — a subject, and a marker saying
# an edge produced it — and knows nothing about API Gateway, a request context or
# a claim (DR-0024). This is where that conversion happens: parameter mapping is
# AWS's own mechanism, so nothing here re-implements AWS behaviour, and it runs in
# the component that is already in front of the service (DR-0025).
#
# `overwrite:` is load-bearing. The service trusts these headers only because
# nothing but the edge can set them, and that is true only because the edge
# replaces whatever the caller sent on every request.
resource "aws_apigatewayv2_integration" "api" {
  api_id                 = aws_apigatewayv2_api.this.id
  integration_type       = "AWS_PROXY"
  integration_uri        = aws_lambda_function.api.invoke_arn
  payload_format_version = "2.0"

  request_parameters = {
    # The authorizer has already run, so this always resolves. If it ever did
    # not, API Gateway would skip the mapping and the subject would simply be
    # absent — which is why the marker below is set separately and statically.
    "overwrite:header.x-auth-subject" = "$context.authorizer.claims.sub"

    # Nothing reads the value; its presence is the assertion. It is what lets the
    # service tell "no edge spoke, so this is a developer" from "an edge spoke
    # and could not name the caller, so refuse" — DR-0025.
    "overwrite:header.x-auth-edge" = "apigateway"
  }
}

# /health runs outside the authorizer, where $context.authorizer.claims.sub
# cannot resolve. A skipped mapping leaves a caller's own header in place, so
# this integration removes both rather than mapping either — the probe reaches
# the service with no AuthContext at all, which is what it should have.
resource "aws_apigatewayv2_integration" "health" {
  api_id                 = aws_apigatewayv2_api.this.id
  integration_type       = "AWS_PROXY"
  integration_uri        = aws_lambda_function.api.invoke_arn
  payload_format_version = "2.0"

  # The value of a `remove:` mapping is a literal pair of single quotes, which is
  # how AWS spells "no value" here. An actual empty string is sent as null, and
  # API Gateway drops the mapping instead of storing it — which plans as a change
  # that applies cleanly and is still pending the next time.
  request_parameters = {
    "remove:header.x-auth-subject" = "''"
    "remove:header.x-auth-edge"    = "''"
  }
}

# Validates Cognito access tokens. `audience` is the app client id, which is
# what a Cognito access token carries in `client_id` and an id token in `aud`.
resource "aws_apigatewayv2_authorizer" "cognito" {
  api_id           = aws_apigatewayv2_api.this.id
  name             = "${local.name}-cognito"
  authorizer_type  = "JWT"
  identity_sources = ["$request.header.Authorization"]

  jwt_configuration {
    issuer   = local.user_pool_issuer
    audience = [local.app_client_id]
  }
}

# Everything the SPA calls sits behind the authorizer. crates/server serves
# /api/dashboard today; {proxy+} means a new endpoint under /api needs no change
# here.
#
# One route per method rather than a single ANY route: ANY matches OPTIONS too,
# which would put the JWT authorizer in front of the CORS preflight — and a
# preflight carries no Authorization header, so it would be answered with a 401
# and the browser would block the request it precedes. Leaving OPTIONS unrouted
# is what lets the HTTP API answer it from cors_configuration above. A new method
# therefore goes in local.api_methods, not here.
resource "aws_apigatewayv2_route" "api" {
  for_each = toset(local.api_methods)

  api_id             = aws_apigatewayv2_api.this.id
  route_key          = "${each.value} /api/{proxy+}"
  target             = "integrations/${aws_apigatewayv2_integration.api.id}"
  authorization_type = "JWT"
  authorizer_id      = aws_apigatewayv2_authorizer.cognito.id
}

# Deliberately unauthenticated: /health exists to be probed, and a probe has no
# token. It returns a constant and reveals nothing.
resource "aws_apigatewayv2_route" "health" {
  api_id    = aws_apigatewayv2_api.this.id
  route_key = "GET /health"
  target    = "integrations/${aws_apigatewayv2_integration.health.id}"
}

resource "aws_cloudwatch_log_group" "api_gateway" {
  name              = "/aws/apigateway/${local.name}"
  retention_in_days = var.log_retention_days
}

# The $default stage serves at the root of the API's own domain, so no stage name
# appears in the URL the SPA is built with.
resource "aws_apigatewayv2_stage" "default" {
  api_id      = aws_apigatewayv2_api.this.id
  name        = "$default"
  auto_deploy = true

  access_log_settings {
    destination_arn = aws_cloudwatch_log_group.api_gateway.arn
    format = jsonencode({
      requestId        = "$context.requestId"
      httpMethod       = "$context.httpMethod"
      path             = "$context.path"
      status           = "$context.status"
      responseLatency  = "$context.responseLatency"
      integrationError = "$context.integrationErrorMessage"
    })
  }
}
