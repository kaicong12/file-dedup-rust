# Terraform Infrastructure Setup

## Prerequisites

1. AWS CLI configured with appropriate credentials
2. Terraform installed
3. Access to the shared S3 bucket for state storage

## Remote State Backend Setup

This project uses S3 for remote state storage to enable team collaboration and prevent resource duplication.

### First-Time Setup

1. **Create the S3 bucket for state storage** (only needs to be done once by one team member):

   ```bash
   aws s3 mb s3://your-terraform-state-bucket --region us-east-1
   ```

2. **Enable versioning on the bucket**:

   ```bash
   aws s3api put-bucket-versioning \
     --bucket your-terraform-state-bucket \
     --versioning-configuration Status=Enabled
   ```

3. **Update the bucket name** in `provider.tf`

### For Team Members

1. **Initialize Terraform** (this will configure the remote backend):

   ```bash
   cd iac
   terraform init
   ```

2. **Create your own tfvars file**:

   ```bash
   cp terraform.tfvars.example terraform.tfvars
   # Edit terraform.tfvars with your specific values
   ```

3. **Plan and apply**:
   ```bash
   terraform plan
   terraform apply
   ```

## Important Notes

- **Never commit `terraform.tfvars`** - it contains sensitive information
- **Always run `terraform init`** when joining the project or when backend configuration changes
- **The state file is now shared** - all team members will see the same infrastructure state
- **State locking prevents concurrent modifications** - only one person can apply changes at a time

## Troubleshooting

If you get state-related errors:

1. Ensure you have access to the S3 bucket
2. Run `terraform init` to reconfigure the backend
3. Check that your AWS credentials are properly configured
