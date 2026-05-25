DO $$
DECLARE
    schema_name text;
BEGIN
    FOR schema_name IN SELECT nspname FROM pg_catalog.pg_namespace WHERE nspname LIKE 'rufs_customer_%' LOOP
        EXECUTE 'SET LOCAL search_path = ' || schema_name;

        -- UPDATE rufs_user set menu = array[
        --     '{"group":"Cadastros","label":"Clientes e Fornecedores","path":"person/search"}',
        --     '{"group":"Cadastros","label":"Código de barras","path":"barcode/search"}',
        --     '{"group":"Cadastros","label":"Produtos","path":"product/search"}',
        --     '{"group":"Cadastros","label":"Serviços","path":"service/search"}',
        --     '{"group":"Cadastros","label":"Contas","path":"account/search"}',
        --     '{"group":"Cadastros","label":"Requisições","path":"request.js/search"}',
        --     '{"group":"Cadastros","label":"Notas fiscais eletrônicas","path":"request_nfe/search"}',

        --     '{"group":"Tabelas","label":"Confaz Cest","path":"confaz_cest/search"}',

        --     '{"group":"Movimento","label":"Vendas","path":"request_product/search"}',
        --     '{"group":"Movimento","label":"Serviços","path":"request_service/search"}',
        --     '{"group":"Movimento","label":"Consertos","path":"request_repair/search"}',
        --     '{"group":"Movimento","label":"Financeiro","path":"request_payment/search"}',
        --     '{"group":"Movimento","label":"Estoque","path":"stock/search"}',

        --     '{"group":"Rotinas","label":"Compra","path":"request/new?instance.type=0&instance.state=10"}',
        --     '{"group":"Rotinas","label":"Venda","path":"request/new?instance.type=1&instance.state=10"}',
        --     '{"group":"Rotinas","label":"Conserto","path":"request/new?instance.type=1&instance.state=10"}',
        --     '{"group":"Rotinas","label":"Importar","path":"nfe_import.js/?instance.type=0&instance.state=10"}',

        --     '{"group":"Ferramentas","label":"Scanner EAN_13/EAN_8","path":"scanner.js/?instance.type=EAN"}',
        --     '{"group":"Ferramentas","label":"Scanner QR_CODE","path":"scanner.js/?instance.type=QR_CODE"}'
        -- ]::jsonb[] where name != 'admin';

        -- UPDATE rufs_user set menu = array[
        --     '{"group":"Cadastros","label":"Usuários","path":"rufs_user/search"}'
        -- ]::jsonb[] where name = 'admin';

        ALTER TABLE rufs_user DROP COLUMN menu;
        ALTER TABLE rufs_user ADD COLUMN menu jsonb NOT NULL DEFAULT '{}'::jsonb;

        UPDATE rufs_user set menu = '{
            "Cadastros": {
                "Clientes e Fornecedores": "person/search",
                "Código de barras": "barcode/search",
                "Produtos": "product/search",
                "Serviços": "service/search",
                "Contas": "account/search",
                "Requisições": "request.js/search",
                "Notas fiscais eletrônicas": "request_nfe/search"
            },
            "Tabelas": {
                "Confaz Cest": "confaz_cest/search"
            },
            "Movimento": {
                "Vendas": "request_product/search",
                "Serviços": "request_service/search",
                "Consertos": "request_repair/search",
                "Financeiro": "request_payment/search",
                "Estoque": "stock/search"
            },
            "Rotinas": {
                "Compra": "request/new?instance.type=0&instance.state=10",
                "Venda": "request/new?instance.type=1&instance.state=10",
                "Conserto": "request/new?instance.type=1&instance.state=10",
                "Importar": "nfe_import.js/?instance.type=0&instance.state=10"
            },
            "Ferramentas": {
                "Scanner EAN_13/EAN_8": "scanner.js/?instance.type=EAN",
                "Scanner QR_CODE": "scanner.js/?instance.type=QR_CODE"
            }
        }'::jsonb where name != 'admin';

        UPDATE rufs_user set menu = '{
            "Cadastros": {
                "Usuários": "rufs_user/search"
            }
        }'::jsonb where name = 'admin';
    END LOOP;
END $$;
