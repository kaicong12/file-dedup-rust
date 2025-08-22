provider "aws" {
  region = "us-east-1"
  profile = "sso_profile"
}

terraform {
  backend "s3" {
    bucket         = "file-dedup-terraform-state-bucket"
    key            = "file-dedup-rust/terraform.tfstate"
    region         = "us-east-1"
    use_lockfile   = true
    profile        = "sso_profile"
  }
}
