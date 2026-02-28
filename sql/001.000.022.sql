INSERT INTO camex_ncm (ncm,descricao,data_fim) VALUES (85171231, 'Portáteis', '2022-03-31');
INSERT INTO camex_ncm (ncm,descricao,data_fim) VALUES (62029300, 'De fibras sintéticas ou artificiais', '2022-03-31');
INSERT INTO camex_ncm (ncm,descricao,data_fim) VALUES (62019300, 'De fibras sintéticas ou artificiais', '2022-03-31');
INSERT INTO camex_ncm (ncm,descricao,data_fim) VALUES (34021190, 'Outros', '2022-03-31');
INSERT INTO camex_ncm (ncm,descricao,data_fim) VALUES (4031000, 'Iogurte', '2022-03-31');
--INSERT INTO camex_ncm (ncm,descricao,data_fim) VALUES (85258029, 'Outras', '2022-03-31');

-- delete from rufs_customer_80803792034.barcode as b1 where exists (select barcode from rufs_customer_80803792034.barcode as b2 where b2.barcode = '0' || b1.barcode);
-- update rufs_customer_80803792034.barcode set barcode = REGEXP_REPLACE(barcode, '0(\d{13})', '\1') where barcode like ('0_____________');

-- delete from rufs_customer_80803792034.barcode as b1 where exists (select barcode from rufs_customer_80803792034.barcode as b2 where b2.barcode = '00000' || b1.barcode);
-- update rufs_customer_80803792034.barcode set barcode = REGEXP_REPLACE(barcode, '00000(\d{8})', '\1') where barcode like ('00000________');

-- update rufs_customer_80803792034.barcode set barcode = REGEXP_REPLACE(barcode, '\d(\d{13})', '\1') where barcode like ('_789__________');
-- update rufs_customer_80803792034.barcode set barcode = REGEXP_REPLACE(barcode, '\d(\d{13})', '\1') where barcode like ('_790__________');

-- delete from rufs_customer_80803792034.barcode where barcode like ('00______');
-- delete from rufs_customer_80803792034.barcode where barcode like ('2____________');

-- UPDATE rufs_customer_80803792034.request_product set product = 168 where product in (3780);
-- DELETE FROM rufs_customer_80803792034.product WHERE id IN (3780);

-- UPDATE rufs_customer_80803792034.request_product set product = 1287 where product in (3503);
-- DELETE FROM rufs_customer_80803792034.product WHERE id IN (3503);

-- UPDATE rufs_customer_80803792034.product set name = REPLACE(name, '+', ' ') where name like '%+%';
-- UPDATE rufs_customer_80803792034.product set name = REPLACE(name, '&AMP;', ' ') where name like '%&AMP;%';
-- UPDATE rufs_customer_80803792034.product set name = TRIM(name) where name like ' %';
-- UPDATE rufs_customer_80803792034.product set name = TRIM(name) where name like '% ';
-- UPDATE rufs_customer_80803792034.product set name = REGEXP_REPLACE(name, '\s{2,}', ' ', 'g') where name like '%  %';
