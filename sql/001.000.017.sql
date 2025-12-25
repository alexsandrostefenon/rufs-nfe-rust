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
        'person',
        'account',
        'product',
        'barcode',
        'service',
        'employed',
        'request',
        'request_nfe',
        'request_freight',
        'request_product',
        'request_service',
        'request_payment',
        'request_payment_group',
        'request_repair',
        'stock_product',
        'stock_service'
        ];

        FOREACH table_name IN ARRAY tables LOOP
            EXECUTE 'ALTER TABLE ' || table_name || ' ADD COLUMN IF NOT EXISTS date_last_change timestamp NOT NULL default CURRENT_TIMESTAMP';
            EXECUTE 'CREATE INDEX IF NOT EXISTS idx_' || table_name || '_date_last_change ON ' || table_name || '(date_last_change)';
        END LOOP;
    END LOOP;
END $$;
