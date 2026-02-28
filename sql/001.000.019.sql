DO $$
DECLARE
    schema_name text;
    tables text[];
    table_name text;
BEGIN
    FOR schema_name IN SELECT nspname FROM pg_catalog.pg_namespace WHERE nspname LIKE 'rufs_customer_%' LOOP
        EXECUTE 'SET LOCAL search_path = ' || schema_name;

        CREATE TABLE config (
            name varchar(50) PRIMARY KEY,
            description varchar(255) UNIQUE,
            value varchar(255),
            rufs_group_owner character varying references rufs_group_owner(name)
        );

        INSERT INTO config (name,value,rufs_group_owner) VALUES ('nfe_import-maybe_missing_payment_years', '2017', 'admin');
    END LOOP;
END $$;
