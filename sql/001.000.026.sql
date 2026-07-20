DO $$
DECLARE
    schema_name text;
    tables text[];
    table_name text;
BEGIN
    FOR schema_name IN SELECT nspname FROM pg_catalog.pg_namespace WHERE nspname LIKE 'rufs_customer_%' LOOP
        EXECUTE 'SET LOCAL search_path = ' || schema_name;

        CREATE TABLE account_balance (
            account integer NOT NULL references account,
            date timestamp NOT NULL,
            balance numeric(9,2) DEFAULT 0.000 NOT NULL,
            PRIMARY KEY(account,date),
        );

        ALTER TABLE rufs_user DROP CONSTRAINT rufs_user_pkey;
        ALTER TABLE rufs_user ADD PRIMARY KEY (name);

        tables := ARRAY[
        'person',
        'account',
        'account_balance',
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

        UPDATE rufs_user set menu['Tabelas'] = menu['Tabelas'] || '{"Acerto Bancário/Caixa": "account_balance/search"}'::jsonb where name != 'admin';
        UPDATE rufs_user set roles = roles || '{"path": "/account_balance", "mask": 31}'::jsonb where name != 'admin';

        FOREACH table_name IN ARRAY tables LOOP
            EXECUTE 'ALTER TABLE ' || table_name || ' ADD COLUMN IF NOT EXISTS user_last_change varchar references rufs_user default '''admin'''';
        END LOOP;
    END LOOP;
END $$;
