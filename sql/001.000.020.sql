DO $$
DECLARE
    schema_name text;
    tables text[];
    table_name text;
BEGIN
    FOR schema_name IN SELECT nspname FROM pg_catalog.pg_namespace WHERE nspname LIKE 'rufs_customer_%' LOOP
        EXECUTE 'SET LOCAL search_path = ' || schema_name;

        UPDATE rufs_user set roles = roles || '{"path": "/config", "mask": 1}'::jsonb where name != 'admin';
        UPDATE rufs_user set roles = roles || '{"path": "/config", "mask": 31}'::jsonb where name = 'admin';
    END LOOP;
END $$;
