SELECT
    c.*,
    MAX(app.appointment),
    COUNT(cou.appointment)
FROM
    customer c
    JOIN extra_customer_data ex ON ex.id = c.id
    LEFT JOIN appointment app ON app.customer = c.id
    AND app.salesman = ex.salesman
    AND app.appointment > '2024-02-12'
    AND app.finish_time IS NULL
    LEFT JOIN appointment cou ON cou.customer = c.id AND cou.salesman=ex.salesman
                AND cou.finish_time IS NOT NULL
WHERE
    c.id = 'IOm6u-iKsS0xNzA3NjI0MDUzMTQzNDEtMTgw'
    GROUP BY c.id, app.id, cou.id