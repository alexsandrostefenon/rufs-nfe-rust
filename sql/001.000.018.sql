DO $$
DECLARE
    schema_name text;
    tables text[];
    table_name text;
BEGIN
    FOR schema_name IN SELECT nspname FROM pg_catalog.pg_namespace WHERE nspname LIKE 'rufs_customer_%' LOOP
        EXECUTE 'SET LOCAL search_path = ' || schema_name;

        update rufs_user set path='request.js/search' where path='request/search';

        tables := ARRAY[
        'request_product'
        ];

        FOREACH table_name IN ARRAY tables LOOP
            EXECUTE 'ALTER TABLE ' || table_name || ' ALTER COLUMN value SET DEFAULT 0.0';
        END LOOP;
    END LOOP;
END $$;
