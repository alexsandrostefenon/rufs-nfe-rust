DROP FUNCTION IF EXISTS public.account_balance_after_date;

CREATE FUNCTION public.account_balance_after_date(_account integer, date_balance_start timestamp, request_payment_id_start integer, diff numeric(9,2), request_type integer) RETURNS integer AS $account_balance_after_date$
DECLARE
    rec_balance RECORD;
    date_balance_end timestamp;
    num_rows_changed integer;
    --search_path text;
BEGIN
    --SELECT current_setting('search_path') INTO search_path;
    --RAISE LOG '[account_balance_after_date] search_path : %', search_path;

	--PERFORM set_config('search_path', schema_name, true);
    date_balance_end = '2999-12-31';

    FOR rec_balance IN SELECT date FROM account_balance WHERE
        account = _account AND date > date_balance_start ORDER BY date asc LIMIT 1
    LOOP
        date_balance_end = rec_balance.date;
    END LOOP;

	IF request_type = 0 THEN
	  	diff := diff * (-1.0);
    END IF;

    --RAISE LOG '[account_balance_after_date] date_balance_start = %, request_payment_id_start = %s, date_balance_end = %, diff = %', date_balance_start, request_payment_id_start, date_balance_end, diff;

    UPDATE request_payment SET balance = balance + diff,date_last_change = CURRENT_TIMESTAMP
    WHERE
        account = _account AND COALESCE(payday,due_date) < date_balance_end AND 
            (
            (COALESCE(payday,due_date) > date_balance_start) OR
            ((COALESCE(payday,due_date) = date_balance_start) AND (id >= request_payment_id_start))
            );
    GET DIAGNOSTICS num_rows_changed = ROW_COUNT;
    --RAISE LOG '[account_balance_after_date] num_rows_changed = %', num_rows_changed;
	RETURN num_rows_changed;
END;
$account_balance_after_date$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION public.request_payment_change() RETURNS TRIGGER AS $request_payment_change$
DECLARE
	request_rec RECORD;
    --search_path text;
BEGIN
    --SELECT current_setting('search_path') INTO search_path;
    --RAISE LOG '[request_payment_change] search_path : %', search_path;
	PERFORM set_config('search_path', TG_TABLE_SCHEMA, true);

	IF TG_OP = 'INSERT' THEN
		SELECT INTO request_rec * FROM request WHERE request.id = NEW.request;
		PERFORM public.account_balance_after_date(NEW.account, COALESCE(NEW.payday,NEW.due_date), NEW.id, NEW.value, request_rec.type);
		RETURN NEW;
	ELSEIF TG_OP = 'UPDATE' THEN
        IF  OLD.due_date != NEW.due_date OR OLD.payday != NEW.payday OR OLD.value != NEW.value OR OLD.account != NEW.account THEN
            SELECT INTO request_rec * FROM request WHERE request.id = OLD.request;
            PERFORM public.account_balance_after_date(OLD.account, COALESCE(OLD.payday,OLD.due_date), OLD.id, OLD.value * (-1.0), request_rec.type);
            PERFORM public.account_balance_after_date(NEW.account, COALESCE(NEW.payday,NEW.due_date), NEW.id, NEW.value, request_rec.type);
        END IF;

		RETURN NEW;
	ELSEIF TG_OP = 'DELETE' THEN
		SELECT INTO request_rec * FROM request WHERE request.id = OLD.request;
		PERFORM public.account_balance_after_date(OLD.account, COALESCE(OLD.payday,OLD.due_date), OLD.id, OLD.value * (-1.0), request_rec.type);
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
    date_balance_prev timestamp;
    --search_path text;
BEGIN
    --SELECT current_setting('search_path') INTO search_path;
    --RAISE LOG '[request_payment_before] search_path : %', search_path;

    PERFORM set_config('search_path', TG_TABLE_SCHEMA, true);
    date_new = COALESCE(NEW.payday,NEW.due_date);
    date_balance_prev = '2000-01-01';

    FOR rec_balance IN SELECT date,balance FROM account_balance WHERE
        account = NEW.account AND date < date_new ORDER BY date desc LIMIT 1 
    LOOP
        NEW.balance = rec_balance.balance;
        date_balance_prev = rec_balance.date;
        --RAISE LOG '[request_payment_before] 1 : date_balance_prev = %, NEW.balance = %', date_balance_prev, NEW.balance;
    END LOOP;

    FOR rec_payment IN SELECT id,COALESCE(payday,due_date) as date_ref,balance FROM request_payment WHERE
        account = NEW.account AND COALESCE(payday,due_date) > date_balance_prev AND ((COALESCE(payday,due_date) < date_new OR (COALESCE(payday,due_date) = date_new AND id <= COALESCE(NEW.id, 999999)))) ORDER BY date_ref desc,id desc LIMIT 1 
    LOOP
        NEW.balance = rec_payment.balance;
        --RAISE LOG '[request_payment_before] 2 : date_balance_prev = %, NEW.balance = %', date_balance_prev, NEW.balance;
    END LOOP;

    RETURN NEW;
END;
$request_payment_before$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION public.on_change_account_balance_before() RETURNS TRIGGER AS $on_change_account_balance_before$
DECLARE
    date_ini timestamp;
    request_payment_id integer;
	rec_ini RECORD;
    --search_path text;
BEGIN
	PERFORM set_config('search_path', TG_TABLE_SCHEMA, true);
    --SELECT current_setting('search_path') INTO search_path;
    --RAISE LOG '[on_change_account_balance_before] search_path : %', search_path;

	IF TG_OP = 'INSERT' THEN
        IF (NEW.date::time = '00:00:00') THEN
            NEW.date = NEW.date + '23:59:59'::time;
        END IF;

        date_ini = NEW.date;
        request_payment_id = 999999;

        FOR rec_ini IN SELECT COALESCE(payday,due_date) as date_ref,id,balance FROM request_payment WHERE
            account = NEW.account AND COALESCE(payday,due_date) > NEW.date ORDER BY date_ref asc,id asc LIMIT 1 
        LOOP
            NEW.difference = rec_ini.balance - NEW.balance;
            date_ini = rec_ini.date_ref;
            request_payment_id = rec_ini.id;
            --RAISE LOG '[on_change_account_balance_before] : date_ini = %, request_payment_id = %', date_ini, request_payment_id;
        END LOOP;

        PERFORM public.account_balance_after_date(NEW.account, date_ini, request_payment_id, NEW.difference, 0);
        RETURN NEW;
	END IF;

    RETURN OLD;
END;
$on_change_account_balance_before$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION public.on_change_request() RETURNS TRIGGER AS $on_change_request$
DECLARE
    rec RECORD;
BEGIN
	PERFORM set_config('search_path', TG_TABLE_SCHEMA, true);

	IF TG_OP = 'INSERT' THEN
		RETURN NEW;
	ELSIF TG_OP = 'UPDATE' THEN
        IF NEW.type != OLD.type THEN
            FOR rec IN SELECT account,min(date_ref) as date_ini,min(id) as id,sum(value) as diff from (select account,COALESCE(payday,due_date) as date_ref,id,value from request_payment where request = OLD.id) group by account
            LOOP
                if NEW.type = 0 and OLD.type = 1 then
                    PERFORM public.account_balance_after_date(rec.account, rec.date_ini, rec.id, rec.diff * 2, 0);
                elsif NEW.type = 1 and OLD.type = 0 then
                    PERFORM public.account_balance_after_date(rec.account, rec.date_ini, rec.id, rec.diff * 2, 1);
                end if;
            END LOOP;
    	END IF;

        RETURN NEW;
	ELSIF TG_OP = 'DELETE' THEN
		RETURN OLD;
	END IF;

	RETURN NULL;
END;
$on_change_request$ LANGUAGE plpgsql;

DO $$
DECLARE
    schema_name text;
BEGIN
    FOR schema_name IN SELECT nspname FROM pg_catalog.pg_namespace WHERE nspname LIKE 'rufs_customer_%' LOOP
        EXECUTE 'SET LOCAL search_path = ' || schema_name;

        ALTER TABLE request_payment ADD COLUMN IF NOT EXISTS date_last_user_change timestamp NOT NULL default CURRENT_TIMESTAMP;
        ALTER TABLE account_balance ADD COLUMN IF NOT EXISTS difference numeric(9,2) DEFAULT 0.00 NOT NULL;

        DROP TRIGGER IF EXISTS trigger_account_balance ON account_balance;
        DROP TRIGGER IF EXISTS trigger_account_balance_before ON account_balance;
        CREATE TRIGGER trigger_account_balance_before BEFORE INSERT OR UPDATE OR DELETE ON account_balance FOR EACH ROW WHEN (pg_trigger_depth() = 0) EXECUTE PROCEDURE public.on_change_account_balance_before();

        DROP TRIGGER IF EXISTS trigger_request ON request;
        CREATE TRIGGER trigger_request AFTER INSERT OR UPDATE OR DELETE ON request FOR EACH ROW WHEN (pg_trigger_depth() = 0) EXECUTE PROCEDURE public.on_change_request();
    END LOOP;
END $$;

DO $$
DECLARE
    schema_name text;
    _account integer;
BEGIN
    FOR schema_name IN SELECT nspname FROM pg_catalog.pg_namespace WHERE nspname LIKE 'rufs_customer_%' LOOP
        EXECUTE 'SET LOCAL search_path = ' || schema_name;
        ALTER TABLE request_payment DISABLE TRIGGER request_payment_trigger;
        ALTER TABLE request_payment DISABLE TRIGGER request_payment_trigger_before;

        FOR _account IN SELECT id FROM account LOOP
            UPDATE request_payment t SET balance = u.value,date_last_change = CURRENT_TIMESTAMP FROM (
                SELECT rp.id,SUM(-rp.value * (-1) ^ r.type) OVER (ORDER BY COALESCE(payday,due_date),rp.id) AS value FROM request_payment rp,request r WHERE r.type in (0,1) AND rp.request = r.id and account = _account
            ) u WHERE t.id = u.id;
        END LOOP;

        ALTER TABLE request_payment ENABLE TRIGGER request_payment_trigger;
        ALTER TABLE request_payment ENABLE TRIGGER request_payment_trigger_before;
    END LOOP;
END $$;

--SELECT COALESCE(payday,due_date) as date_ref,rp.id,balance,r.type,(rp.value * (-1) ^ r.type) as value FROM rufs_customer_80803792034.request_payment rp,rufs_customer_80803792034.request r WHERE r.type in (0,1) AND rp.request = r.id and account = 1 ORDER BY date_ref desc,rp.id desc;
