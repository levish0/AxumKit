# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.3] - 2026-01-26

### Changed

- Upgrade Rust version from 1.92.0 to 1.93.0
- Update `sea-orm` from 2.0.0-rc.28 to 2.0.0-rc.29
- Add v4 feature to `uuid` crate

### Improved

- Parallelize E2E tests
  - Each test run gets a unique project name for container isolation
  - File-based locking for coordinating image builds across test binaries
  - Use `docker compose` service names instead of container names for port lookup

## [0.2.2] - 2025-01-20

### Removed

- S3 checksum calculation/validation for SeaweedFS and R2 connections
  - Removed `RequestChecksumCalculation::WhenRequired` from SeaweedFS client
  - Removed `RequestChecksumCalculation::WhenRequired` and `ResponseChecksumValidation::WhenRequired` from R2 client
  - Applies to both `axumkit-server` and `axumkit-worker`
