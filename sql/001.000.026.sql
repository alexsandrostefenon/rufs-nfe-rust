CREATE OR REPLACE FUNCTION public.account_balance_after_date(--schema_name name, 
    _account integer, date_ref timestamp, ref_id integer, diff numeric(9,2), request_type integer) RETURNS integer AS $account_balance_after_date$
DECLARE
    rec_balance RECORD;
    date_balance timestamp;
BEGIN
	--PERFORM set_config('search_path', schema_name, true);
    date_balance = '2999-12-31';

    FOR rec_balance IN SELECT date FROM account_balance WHERE
        account = _account AND date > date_ref ORDER BY date desc LIMIT 1
    LOOP
        date_balance = rec_balance.date;
    END LOOP;

	IF request_type = 0 THEN
	  		diff := diff * (-1.0);
    END IF;

    UPDATE request_payment SET balance = balance + diff
    WHERE
        account = _account AND GREATEST(payday,due_date) < date_balance AND 
            (
            GREATEST(payday,due_date) > date_ref OR
            (GREATEST(payday,due_date) = date_ref AND id >= ref_id)
            );
	RETURN 1;
END;
$account_balance_after_date$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION public.request_payment_change() RETURNS TRIGGER AS $request_payment_change$
DECLARE
	request_rec RECORD;
BEGIN
	PERFORM set_config('search_path', TG_TABLE_SCHEMA, true);

	IF (TG_OP = 'INSERT') THEN
		SELECT INTO request_rec * FROM request WHERE request.id = NEW.request;
		PERFORM public.account_balance_after_date(--TG_TABLE_SCHEMA,
        NEW.account, GREATEST(NEW.due_date, NEW.payday), NEW.id, NEW.value, request_rec.type);
		RETURN NEW;
	ELSEIF (TG_OP = 'UPDATE') THEN
		SELECT INTO request_rec * FROM request WHERE request.id = OLD.request;
		PERFORM public.account_balance_after_date(--TG_TABLE_SCHEMA, 
        OLD.account, GREATEST(OLD.due_date, OLD.payday), OLD.id, OLD.value * (-1.0), request_rec.type);
		PERFORM public.account_balance_after_date(--TG_TABLE_SCHEMA, 
        NEW.account, GREATEST(NEW.due_date, NEW.payday), NEW.id, NEW.value, request_rec.type);
		RETURN NEW;
	ELSEIF (TG_OP = 'DELETE') THEN
		SELECT INTO request_rec * FROM request WHERE request.id = OLD.request;
		PERFORM public.account_balance_after_date(--TG_TABLE_SCHEMA, 
        OLD.account, GREATEST(OLD.due_date, OLD.payday), OLD.id, OLD.value * (-1.0), request_rec.type);
		RETURN OLD;
	END IF;

	RETURN NULL;
END;
$request_payment_change$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION public.request_payment_before() RETURNS TRIGGER AS $request_payment_before$
DECLARE
    rec_balance RECORD;
    rec_payment RECORD;
    date_new timestamp;
    date_balance timestamp;
BEGIN
    PERFORM set_config('search_path', TG_TABLE_SCHEMA, true);
    date_new = GREATEST(NEW.payday,NEW.due_date);
    date_balance = '2000-01-01';

    FOR rec_balance IN SELECT date,balance FROM account_balance WHERE
        account = NEW.account AND date <= date_new ORDER BY date desc LIMIT 1 
    LOOP
        NEW.balance = rec_balance.balance;
        date_balance = rec_balance.date;
    END LOOP;

    FOR rec_payment IN SELECT id,GREATEST(payday,due_date) as date_ref,balance FROM request_payment WHERE
        account = NEW.account AND GREATEST(payday,due_date) > date_balance AND GREATEST(payday,due_date) <= date_new ORDER BY date_ref desc,id desc LIMIT 1 
    LOOP
        NEW.balance = rec_payment.balance;
    END LOOP;

    RETURN NEW;
END;
$request_payment_before$ LANGUAGE plpgsql;

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
            balance numeric(9,2) DEFAULT 0.00 NOT NULL,
            date_last_change timestamp DEFAULT CURRENT_TIMESTAMP NOT NULL,
            PRIMARY KEY(account,date)
        );

        CREATE INDEX IF NOT EXISTS idx_account_balance_date_last_change ON account_balance(date_last_change);

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
            EXECUTE 'ALTER TABLE ' || table_name || ' ADD COLUMN IF NOT EXISTS user_last_change varchar references rufs_user default ''admin''';
        END LOOP;

        DROP TRIGGER request_payment_trigger_before ON request_payment; 
        CREATE TRIGGER request_payment_trigger_before BEFORE INSERT OR UPDATE ON request_payment FOR EACH ROW WHEN (pg_trigger_depth() = 0) EXECUTE PROCEDURE public.request_payment_before();
        DROP FUNCTION request_payment_before;
        DROP TRIGGER request_payment_trigger ON request_payment;
        CREATE TRIGGER request_payment_trigger AFTER INSERT OR UPDATE OR DELETE ON request_payment FOR EACH ROW WHEN (pg_trigger_depth() = 0) EXECUTE PROCEDURE public.request_payment_change();
        DROP FUNCTION request_payment_change;
        DROP FUNCTION account_balance_after_date;
    END LOOP;
END $$;
