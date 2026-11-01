DO $$
DECLARE
    runtime_role text := current_setting('hirune.runtime_role');
    maintenance_role text := current_setting('hirune.maintenance_role');
    selected_role text;
    relation record;
    permission_list text;
    forbidden_permission text;
BEGIN
    IF runtime_role = maintenance_role THEN
        RAISE EXCEPTION 'Runtime and maintenance roles must differ';
    END IF;
    FOREACH selected_role IN ARRAY ARRAY[runtime_role, maintenance_role] LOOP
        IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = selected_role)
           OR selected_role = current_user
           OR EXISTS (SELECT FROM pg_roles WHERE rolname = selected_role
                      AND (rolsuper OR rolcreatedb OR rolcreaterole OR rolbypassrls OR rolreplication))
           OR EXISTS (SELECT FROM pg_auth_members member JOIN pg_roles role ON role.oid = member.member
                      WHERE role.rolname = selected_role)
           OR EXISTS (SELECT FROM pg_database database JOIN pg_roles role ON role.oid = database.datdba
                      WHERE database.datname = current_database() AND role.rolname = selected_role)
           OR EXISTS (SELECT FROM pg_namespace namespace JOIN pg_roles role ON role.oid = namespace.nspowner
                      WHERE namespace.nspname = 'public' AND role.rolname = selected_role)
           OR has_database_privilege(selected_role, current_database(), 'CREATE')
           OR has_schema_privilege(selected_role, 'public', 'CREATE') THEN
            RAISE EXCEPTION 'Grant targets must be existing unprivileged roles without inherited roles';
        END IF;
        EXECUTE format('GRANT USAGE ON SCHEMA public TO %I', selected_role);
        EXECUTE format('ALTER DEFAULT PRIVILEGES IN SCHEMA public REVOKE ALL ON TABLES FROM %I', selected_role);
        EXECUTE format('ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT ON TABLES TO %I', selected_role);
        FOR relation IN SELECT tablename, tableowner FROM pg_tables WHERE schemaname = 'public' LOOP
            IF relation.tableowner <> current_user THEN
                RAISE EXCEPTION 'Migration role must own application tables';
            END IF;
            permission_list := CASE
                WHEN relation.tablename = 'seaql_migrations' THEN 'SELECT'
                WHEN relation.tablename IN ('audit_events', 'api_usage')
                    THEN CASE WHEN selected_role = maintenance_role THEN 'SELECT, INSERT, DELETE' ELSE 'SELECT, INSERT' END
                ELSE 'SELECT, INSERT, UPDATE, DELETE'
            END;
            EXECUTE format('REVOKE ALL ON TABLE public.%I FROM %I', relation.tablename, selected_role);
            EXECUTE format('GRANT %s ON TABLE public.%I TO %I', permission_list, relation.tablename, selected_role);
            FOREACH forbidden_permission IN ARRAY ARRAY['TRUNCATE', 'TRIGGER', 'REFERENCES'] LOOP
                IF has_table_privilege(selected_role, format('public.%I', relation.tablename), forbidden_permission) THEN
                    RAISE EXCEPTION 'Unexpected effective table privileges remain';
                END IF;
            END LOOP;
            IF has_any_column_privilege(selected_role, format('public.%I', relation.tablename), 'REFERENCES') THEN
                RAISE EXCEPTION 'Unexpected effective column privileges remain';
            END IF;
            IF relation.tablename = 'seaql_migrations' AND
               (has_table_privilege(selected_role, 'public.seaql_migrations', 'INSERT, UPDATE, DELETE') OR
                has_any_column_privilege(selected_role, 'public.seaql_migrations', 'INSERT, UPDATE')) THEN
                RAISE EXCEPTION 'Migration history must remain read-only';
            END IF;
            IF relation.tablename IN ('audit_events', 'api_usage') AND
               (has_any_column_privilege(selected_role, format('public.%I', relation.tablename), 'UPDATE') OR
                (selected_role = runtime_role AND has_table_privilege(selected_role, format('public.%I', relation.tablename), 'DELETE'))) THEN
                RAISE EXCEPTION 'Audit and usage mutation privileges remain';
            END IF;
        END LOOP;
    END LOOP;
END $$;
