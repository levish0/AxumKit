#!/bin/sh
set -eu
# Role administration is separate from application schema migrations.
psql -X --username postgres --dbname postgres --set ON_ERROR_STOP=1 <<'SQL'
\getenv owner_password SCHEMA_OWNER_PASSWORD
\getenv runtime_password RUNTIME_PASSWORD
\getenv maintenance_password MAINTENANCE_PASSWORD
CREATE ROLE schema_owner LOGIN PASSWORD :'owner_password';
CREATE ROLE runtime LOGIN PASSWORD :'runtime_password';
CREATE ROLE maintenance LOGIN PASSWORD :'maintenance_password';
\getenv application_database APPLICATION_DATABASE
CREATE DATABASE :"application_database" OWNER schema_owner;
REVOKE ALL ON DATABASE :"application_database" FROM PUBLIC;
GRANT CONNECT ON DATABASE :"application_database" TO runtime, maintenance;
\connect :application_database
REVOKE CREATE ON SCHEMA public FROM PUBLIC;
GRANT USAGE ON SCHEMA public TO runtime, maintenance;
ALTER DEFAULT PRIVILEGES FOR ROLE schema_owner IN SCHEMA public
    GRANT SELECT ON TABLES TO runtime, maintenance;
SQL
