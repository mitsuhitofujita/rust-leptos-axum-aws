# ---------------------------------------------------------------------------
# The application table
#
# One table holds both entities. `pk` is the owner, `sk` carries the entity kind
# and its ordering, so one user's action types and action records live in one
# partition and every screen the design describes is a sort-key query against it.
# The key encoding is the interface, not this file: docs/design/persistence.md.
#
# As expensive to destroy as the Cognito user pool, and for the same reason —
# nothing recreates what is in it. Hence both guards, matching identity:
# prevent_destroy stops Terraform, deletion_protection stops the console and the
# CLI too. DR-0005.
# ---------------------------------------------------------------------------

resource "aws_dynamodb_table" "app" {
  name = "${var.project}-app"

  # One environment, no traffic baseline, and a table that is idle whenever
  # nobody is using the SPA. Nothing here is worth provisioning capacity for.
  billing_mode = "PAY_PER_REQUEST"

  hash_key  = "pk"
  range_key = "sk"

  attribute {
    name = "pk"
    type = "S"
  }

  attribute {
    name = "sk"
    type = "S"
  }

  # Only key attributes are declared. Everything else on an item — name, unit,
  # icon, value, recorded_at — is schemaless and belongs to the application, so
  # adding one is not an infrastructure change.

  # No global secondary index. Every access pattern in
  # docs/design/page-layouts.md is answered inside a single user's partition, and
  # an index costs storage and write throughput for queries nothing issues.

  point_in_time_recovery {
    enabled = var.point_in_time_recovery
  }

  deletion_protection_enabled = true

  lifecycle {
    prevent_destroy = true
  }
}
