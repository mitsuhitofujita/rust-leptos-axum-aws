locals {
  # The methods the SPA calls. The CORS configuration and the routes below both
  # derive from this list, because the two have to agree: a method the routes
  # accept and CORS does not is blocked in the browser, and the reverse is a 404
  # the preflight does not predict.
  api_methods = ["GET", "POST", "PUT", "DELETE"]
}

resource "aws_apigatewayv2_api" "this" {
  name          = local.name
  protocol_type = "HTTP"

  # DR-0001 records the missing CORS layer in crates/server as a gap deployment
  # has to close; it is closed here rather than in the service. An HTTP API
  # answers preflight itself, ahead of the route table, but only for an OPTIONS
  # request no route matches — hence the enumerated routes below.
  cors_configuration {
    allow_origins = [local.cloudfront_url]
    allow_methods = concat(local.api_methods, ["OPTIONS"])
    allow_headers = ["authorization", "content-type"]
    max_age       = 3600
  }
}

# One integration for the whole function. crates/server verifies the
# Authorization header itself now — DR-0028 — so nothing here maps, overwrites
# or removes a header; the request reaches the function exactly as the caller
# sent it, on every route including /health.
resource "aws_apigatewayv2_integration" "api" {
  api_id                 = aws_apigatewayv2_api.this.id
  integration_type       = "AWS_PROXY"
  integration_uri        = aws_lambda_function.api.invoke_arn
  payload_format_version = "2.0"
}

# Everything the SPA calls, verified by crates/server itself. It serves
# /api/dashboard today; {proxy+} means a new endpoint under /api needs no
# change here.
#
# One route per method rather than a single ANY route: ANY matches OPTIONS
# too, and an HTTP API only answers preflight itself from cors_configuration
# above for an OPTIONS request no route matches — an ANY route would instead
# proxy every preflight to a function with no OPTIONS handler, failing the
# request the preflight precedes. This held before DR-0028 for a second reason
# — the JWT authorizer would have intercepted OPTIONS too — which is gone now;
# this one is not. A new method therefore goes in local.api_methods, not here.
resource "aws_apigatewayv2_route" "api" {
  for_each = toset(local.api_methods)

  api_id    = aws_apigatewayv2_api.this.id
  route_key = "${each.value} /api/{proxy+}"
  target    = "integrations/${aws_apigatewayv2_integration.api.id}"
}

# Deliberately unauthenticated: /health exists to be probed, and a probe has
# no token. crates/server's own handler takes no Owner and reveals nothing;
# nothing about this route needs to differ from /api's any more.
resource "aws_apigatewayv2_route" "health" {
  api_id    = aws_apigatewayv2_api.this.id
  route_key = "GET /health"
  target    = "integrations/${aws_apigatewayv2_integration.api.id}"
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
