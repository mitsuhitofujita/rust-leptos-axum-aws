# DR-0006: The Terraform state backend is configured from outside the repository

Status: accepted
Date: 2026-08-02

## Context

DR-0005 put each layer's state in the bucket that `bootstrap` creates. Writing
the configuration turned that sentence into three questions that a design
document cannot answer, because they are all properties of Terraform's backend
mechanism rather than of the infrastructure:

- **A `backend` block accepts neither variables nor interpolation.** Every value
  in it is a literal. The bucket name is therefore a literal in three layers,
  and S3 bucket names are globally unique, so it needs a distinguishing suffix.
  The conventional suffix is the AWS account id.
- **Terraform will not lock S3 state by itself** unless told how. The idiom this
  project would have inherited is a DynamoDB table created alongside the bucket.
- **`bootstrap` cannot start with the backend it ends with.** Its first apply
  creates the bucket its state is supposed to live in, so the configuration
  committed to the repository describes a state it is only ever in once.

The first question acquired an additional constraint: the account id is not to
appear in this repository.

## Decision

**The bucket name lives in `infra/backend.hcl`, which is not committed.** Each
layer's `backend "s3"` block declares only its `key`; `bucket`, `region`,
`encrypt` and `use_lockfile` arrive through
`terraform init -backend-config=../backend.hcl`. `infra/backend.hcl.example` is
committed as the template, and `bootstrap` renders the real contents as an
output so the operator copies rather than derives them:

```sh
terraform -chdir=infra/bootstrap output -raw backend_hcl > infra/backend.hcl
```

The bucket is `<project>-tfstate-<account_id>`, assembled inside `bootstrap`
from `data.aws_caller_identity`. No configuration file contains the account id.

**Locking uses the S3 native lock file**, `use_lockfile = true`. There is no
DynamoDB table.

**`bootstrap` ships with a local backend and a documented migration.** The
committed tree contains no `backend.tf` for it, only `backend.tf.example`.
Creating the bucket and then moving state into it is:

```sh
terraform -chdir=infra/bootstrap init
terraform -chdir=infra/bootstrap apply
terraform -chdir=infra/bootstrap output -raw backend_hcl > infra/backend.hcl
cp infra/bootstrap/backend.tf.example infra/bootstrap/backend.tf
terraform -chdir=infra/bootstrap init -migrate-state -backend-config=../backend.hcl
```

`infra/bootstrap/backend.tf` is gitignored, so the repository always shows
`bootstrap` in the shape a first-time operator needs.

## Alternatives

**A DynamoDB lock table.** The older and far more widely documented idiom, and
rejected on three counts. It is a second resource that has to exist before any
layer can be applied, which enlarges the bootstrap problem this project already
has one of. The S3 backend now documents `dynamodb_table` as deprecated and
slated for removal in a future minor version, so adopting it would be adopting
something already on its way out. And it buys nothing: `use_lockfile` is
supported by OpenTofu as well as Terraform, so it does not spend the escape
hatch DR-0004 deliberately kept open.

**Writing the bucket name into the repository.** One command shorter for every
operator, and the thing most projects do. Rejected because the name carries the
AWS account id. An account id is not a credential and knowing one grants nobody
anything, but it is an identifier of the account rather than of the project, and
this repository is about the project.

**Deriving the bucket name from something already public.** Considered — a hash
of the project name, or a random suffix stored in SSM — and rejected. A hash is
unreadable in a console listing, and a random suffix has to be discovered
somewhere before the first `init` anyway, which is the problem `backend.hcl`
already solves without inventing a naming scheme.

**A `just` recipe that reads the bucket from SSM at init time.** Attractive,
because `bootstrap` publishes the name to `/<project>/bootstrap/state_bucket`
and nothing would need copying. Rejected as the primary mechanism: it makes
every `terraform init` depend on an AWS API call and on the CLI's credentials
being present, for a value that changes approximately never. The parameter is
still published, because being able to find the bucket without reading state is
worth having.

**Keeping `bootstrap` on a local backend permanently**, with its state file
committed or kept on the operator's disk. Rejected. A state file is not a thing
to commit, and one held on a single machine is a single point of failure for
the layer whose loss DR-0005 rates as the most expensive.

**A `backend.tf` committed for `bootstrap` and worked around at first init.**
Terraform offers `-backend=false`, and the first apply could be made to run
under it. Rejected because it depends on how `-backend=false` interacts with an
uninitialised working directory, which is not what that flag documents itself as
being for. A file that is copied into place is dull and obvious, and a
first-time operator can see what it does.

## Consequences

Easy: the account id stays out of the repository; there is one fewer resource
to create, protect, and pay for; locking is configuration rather than
infrastructure; and a reader of `infra/bootstrap` sees the layer as it is on the
day it is first applied rather than as it is afterwards.

Hard, and accepted deliberately:

- **`terraform init` is not the bare command.** Every layer but `bootstrap`
  needs `-backend-config=../backend.hcl`, and forgetting it produces a prompt
  for the bucket rather than an error. `just tf-init <layer>` exists so the flag
  is not remembered.
- **A fresh clone cannot plan anything.** `infra/backend.hcl` is absent until
  someone creates it, so a new machine's first step is always to fetch or
  regenerate it. This is the cost of the account id not being in the tree, paid
  once per machine.
- **The migration is a manual sequence run once**, and a wrong `init` in the
  middle of it is recoverable only from the local state file. It is written
  down in `docs/design/deployment.md` and in `backend.tf.example` itself.
- **`use_lockfile` is newer than the DynamoDB idiom**, so it has less public
  troubleshooting material behind it. It requires Terraform 1.11 or later,
  which is why `required_version` names that floor.

Reversing any of this is cheap. Committing the bucket name is deleting two
gitignore lines. Adding a DynamoDB table is a resource in `bootstrap` and one
more argument in `backend.hcl`, and it can be done without touching state.
