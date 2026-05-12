.mode tabs
.headers on

DROP TABLE IF EXISTS temp.t143_domain_anchors;
CREATE TEMP TABLE t143_domain_anchors (
    target_type_name TEXT NOT NULL,
    ref_scope TEXT NOT NULL,
    classification TEXT NOT NULL,
    source_domain TEXT NOT NULL,
    evidence TEXT NOT NULL
);

INSERT INTO temp.t143_domain_anchors (
    target_type_name,
    ref_scope,
    classification,
    source_domain,
    evidence
) VALUES
    ('Строка', 'non_query_ref', 'likely_bsl_language_fact', 'BslLanguage', 'shlang:def_String'),
    ('String', 'non_query_ref', 'likely_bsl_language_fact', 'BslLanguage', 'shlang:def_String'),
    ('Булево', 'non_query_ref', 'likely_bsl_language_fact', 'BslLanguage', 'shlang:def_Boolean'),
    ('Boolean', 'non_query_ref', 'likely_bsl_language_fact', 'BslLanguage', 'shlang:def_Boolean'),
    ('Число', 'non_query_ref', 'likely_bsl_language_fact', 'BslLanguage', 'shlang:def_Number'),
    ('Number', 'non_query_ref', 'likely_bsl_language_fact', 'BslLanguage', 'shlang:def_Number'),
    ('Дата', 'non_query_ref', 'likely_bsl_language_fact', 'BslLanguage', 'shlang:def_Date'),
    ('Date', 'non_query_ref', 'likely_bsl_language_fact', 'BslLanguage', 'shlang:def_Date'),
    ('Неопределено', 'non_query_ref', 'likely_bsl_language_fact', 'BslLanguage', 'shlang:def_Undefined'),
    ('Undefined', 'non_query_ref', 'likely_bsl_language_fact', 'BslLanguage', 'shlang:def_Undefined'),
    ('Тип', 'non_query_ref', 'likely_bsl_language_fact', 'BslLanguage', 'shlang:def_Type'),
    ('Type', 'non_query_ref', 'likely_bsl_language_fact', 'BslLanguage', 'shlang:def_Type'),
    ('Строка', 'query_ref', 'likely_query_language_or_skd_fact', 'QueryLanguage', 'shquery:LitString'),
    ('String', 'query_ref', 'likely_query_language_or_skd_fact', 'QueryLanguage', 'shquery:LitString'),
    ('Булево', 'query_ref', 'likely_query_language_or_skd_fact', 'QueryLanguage', 'shquery:TRUE/FALSE'),
    ('Boolean', 'query_ref', 'likely_query_language_or_skd_fact', 'QueryLanguage', 'shquery:TRUE/FALSE'),
    ('Число', 'query_ref', 'likely_query_language_or_skd_fact', 'QueryLanguage', 'shquery numeric value domain'),
    ('Number', 'query_ref', 'likely_query_language_or_skd_fact', 'QueryLanguage', 'shquery numeric value domain'),
    ('Дата', 'query_ref', 'likely_query_language_or_skd_fact', 'QueryLanguage', 'shquery:LitDate'),
    ('Date', 'query_ref', 'likely_query_language_or_skd_fact', 'QueryLanguage', 'shquery:LitDate'),
    ('Null', 'query_ref', 'likely_query_language_or_skd_fact', 'QueryLanguage', 'shquery:NULL'),
    ('NULL', 'query_ref', 'likely_query_language_or_skd_fact', 'QueryLanguage', 'shquery:NULL');

DROP VIEW IF EXISTS temp.t143_unresolved_classified;
CREATE TEMP VIEW t143_unresolved_classified AS
SELECT
    COALESCE(
        anchor.classification,
        CASE
            WHEN r.target_type_name LIKE '%информационной базы%'
                OR r.target_type_name LIKE '%information base%'
                OR r.target_type_name LIKE '%макет%'
                OR r.target_type_name LIKE '%template%'
            THEN 'configuration_or_source_code_downstream_provider'
            WHEN r.ref_kind IN ('query_field_type', 'query_parameter_type')
            THEN 'likely_query_language_or_skd_fact'
            ELSE 'still_unclassified_platform_source_gap'
        END
    ) AS classification,
    COALESCE(anchor.source_domain, '') AS source_domain,
    COALESCE(anchor.evidence, '') AS evidence,
    r.ref_kind,
    r.target_type_name,
    r.source_document_id,
    d.kind AS source_kind,
    d.name_primary,
    d.name_alias,
    d.owner_primary,
    d.owner_alias
FROM type_refs r
JOIN documents d ON d.id = r.source_document_id
LEFT JOIN temp.t143_domain_anchors anchor
    ON anchor.target_type_name = r.target_type_name
   AND (
        anchor.ref_scope = 'any'
        OR (anchor.ref_scope = 'query_ref'
            AND r.ref_kind IN ('query_field_type', 'query_parameter_type'))
        OR (anchor.ref_scope = 'non_query_ref'
            AND r.ref_kind NOT IN ('query_field_type', 'query_parameter_type'))
   )
WHERE r.target_resolution_status = 'unresolved';

.print t143_domain_classification_summary
SELECT
    classification,
    COUNT(*) AS rows,
    COUNT(DISTINCT target_type_name) AS distinct_target_names
FROM temp.t143_unresolved_classified
GROUP BY classification
ORDER BY rows DESC, classification;

.print t143_domain_classification_by_role
SELECT
    classification,
    ref_kind,
    COUNT(*) AS rows,
    COUNT(DISTINCT target_type_name) AS distinct_target_names
FROM temp.t143_unresolved_classified
GROUP BY classification, ref_kind
ORDER BY classification, rows DESC, ref_kind;

.print t143_domain_classification_top_names
SELECT
    classification,
    target_type_name,
    ref_kind,
    COUNT(*) AS rows,
    MIN(NULLIF(evidence, '')) AS evidence,
    MIN(source_document_id) AS example_source_document_id
FROM temp.t143_unresolved_classified
GROUP BY classification, target_type_name, ref_kind
ORDER BY classification, rows DESC, target_type_name, ref_kind
LIMIT 80;
