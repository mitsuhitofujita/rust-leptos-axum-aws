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

resource "aws_apigatewayv2_integration" "lambda" {
  api_id                 = aws_apigatewayv2_api.this.id
  integration_type       = "AWS_PROXY"
  integration_uri        = aws_lambda_function.api.invoke_arn
  payload_format_version = "2.0"
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
  target             = "integrations/${aws_apigatewayv2_integration.lambda.id}"
  authorization_type = "JWT"
  authorizer_id      = aws_apigatewayv2_authorizer.cognito.id
}

# Deliberately unauthenticated: /health exists to be probed, and a probe has no
# token. It returns a constant and reveals nothing.
resource "aws_apigatewayv2_route" "health" {
  api_id    = aws_apigatewayv2_api.this.id
  route_key = "GET /health"
  target    = "integrations/${aws_apigatewayv2_integration.lambda.id}"
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
