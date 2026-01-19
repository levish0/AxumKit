# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.2] - 2025-01-20

### Removed

- S3 checksum calculation/validation for SeaweedFS and R2 connections
  - Removed `RequestChecksumCalculation::WhenRequired` from SeaweedFS client
  - Removed `RequestChecksumCalculation::WhenRequired` and `ResponseChecksumValidation::WhenRequired` from R2 client
  - Applies to both `axumkit-server` and `axumkit-worker`
