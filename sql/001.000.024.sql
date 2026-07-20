DO $$
DECLARE
    schema_name text;
BEGIN
    FOR schema_name IN SELECT nspname FROM pg_catalog.pg_namespace WHERE nspname LIKE 'rufs_customer_%' LOOP
        EXECUTE 'SET LOCAL search_path = ' || schema_name;
        ALTER TABLE product ALTER COLUMN unity TYPE varchar(6);
    END LOOP;
END $$;
