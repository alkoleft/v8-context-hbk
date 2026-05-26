use std::hint::black_box;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::{env, fs};

use syntax_helper_search::{
    HbkFactRef, HbkFactSnapshot, HbkFactSnapshotMemoryEntry, HbkGlobalFactKind, HbkLanguageDomain,
    HbkTypeMemberKind,
};

const DEFAULT_ITERATIONS: usize = 20_000;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: measure_hbk_fact_snapshot <index.sqlite> [iterations] [cache.bin]");
        std::process::exit(2);
    };
    let iterations = args
        .next()
        .map(|value| value.parse())
        .transpose()?
        .unwrap_or(DEFAULT_ITERATIONS);
    let cache_path = args.next().map(PathBuf::from);

    let report = HbkFactSnapshot::from_path_with_stage_timings(PathBuf::from(path))?;
    let snapshot = report.snapshot;
    let timings = report.timings;

    println!("snapshot_build_ms={}", timings.total.as_millis());
    print_duration("stage.open_index", timings.open_index);
    print_duration("stage.read_sql_rows", timings.read_sql_rows);
    print_duration("stage.build_lookup_maps", timings.build_lookup_maps);
    print_duration("stage.build_platform_types", timings.build_platform_types);
    print_duration("stage.group_type_refs", timings.group_type_refs);
    print_duration("stage.build_signatures", timings.build_signatures);
    print_duration("stage.build_fact_arenas", timings.build_fact_arenas);
    print_duration(
        "stage.build_fact_ids_relations_availability",
        timings.build_fact_ids_relations_availability,
    );
    print_duration(
        "stage.sort_secondary_indexes",
        timings.sort_secondary_indexes,
    );
    print_duration("stage.assemble_snapshot", timings.assemble_snapshot);

    if let Some(cache_path) = cache_path {
        let write_start = Instant::now();
        snapshot.write_experimental_binary_cache(&cache_path)?;
        let write_elapsed = write_start.elapsed();
        let cache_bytes = fs::metadata(&cache_path)?.len();

        let read_start = Instant::now();
        let cached_snapshot = HbkFactSnapshot::from_experimental_binary_cache(&cache_path)?;
        let read_elapsed = read_start.elapsed();
        let roundtrip_equal = cached_snapshot == snapshot;

        println!("binary_cache.bytes={cache_bytes}");
        print_duration("binary_cache.write", write_elapsed);
        print_duration("binary_cache.read", read_elapsed);
        println!(
            "binary_cache.snapshot_heap_bytes={}",
            cached_snapshot.estimated_heap_bytes()
        );
        println!("binary_cache.roundtrip_equal={roundtrip_equal}");
    }

    let counts = snapshot.counts();
    let memory = snapshot.memory_accounting();
    let handle = snapshot.worker_handle();
    println!("snapshot_heap_bytes={}", memory.total_bytes());
    println!("strings={}", counts.strings);
    println!("platform_types={}", counts.platform_types);
    println!("type_members={}", counts.type_members);
    println!("callables={}", counts.callables);
    println!("globals={}", counts.globals);
    println!("query_tables={}", counts.query_tables);
    println!("query_fields={}", counts.query_fields);
    println!("query_parameters={}", counts.query_parameters);
    println!("language_facts={}", counts.language_facts);
    print_entry("heap.string_store", memory.string_store);
    print_entry("heap.node_arenas", memory.node_arenas);
    print_entry("index.fact_ids", memory.indexes.fact_ids);
    print_entry("index.platform_type_ids", memory.indexes.platform_type_ids);
    print_entry(
        "index.platform_type_names",
        memory.indexes.platform_type_names,
    );
    print_entry(
        "index.platform_type_templates",
        memory.indexes.platform_type_templates,
    );
    print_entry("index.member_ids", memory.indexes.member_ids);
    print_entry("index.members_by_owner", memory.indexes.members_by_owner);
    print_entry(
        "index.members_by_owner_name",
        memory.indexes.members_by_owner_name,
    );
    print_entry(
        "index.members_by_owner_name_kind",
        memory.indexes.members_by_owner_name_kind,
    );
    print_entry("index.callable_ids", memory.indexes.callable_ids);
    print_entry(
        "index.callables_by_owner",
        memory.indexes.callables_by_owner,
    );
    print_entry(
        "index.callables_by_owner_name",
        memory.indexes.callables_by_owner_name,
    );
    print_entry(
        "index.constructors_by_type",
        memory.indexes.constructors_by_type,
    );
    print_entry("index.global_names", memory.indexes.global_names);
    print_entry(
        "index.globals_by_domain_name_kind",
        memory.indexes.globals_by_domain_name_kind,
    );
    print_entry(
        "index.module_event_names",
        memory.indexes.module_event_names,
    );
    print_entry(
        "index.module_contexts_by_domain_language_kind",
        memory.indexes.module_contexts_by_domain_language_kind,
    );
    print_entry("index.query_table_ids", memory.indexes.query_table_ids);
    print_entry("index.query_table_names", memory.indexes.query_table_names);
    print_entry(
        "index.query_table_syntax_names",
        memory.indexes.query_table_syntax_names,
    );
    print_entry(
        "index.query_table_identifiers",
        memory.indexes.query_table_identifiers,
    );
    print_entry(
        "index.query_fields_by_table",
        memory.indexes.query_fields_by_table,
    );
    print_entry(
        "index.query_fields_by_table_name",
        memory.indexes.query_fields_by_table_name,
    );
    print_entry(
        "index.query_parameters_by_table",
        memory.indexes.query_parameters_by_table,
    );
    print_entry(
        "index.query_parameters_by_table_name",
        memory.indexes.query_parameters_by_table_name,
    );
    print_entry("index.language_ids", memory.indexes.language_ids);
    print_entry("index.language_names", memory.indexes.language_names);
    print_entry(
        "index.availability_by_fact",
        memory.indexes.availability_by_fact,
    );
    print_entry(
        "index.availability_since_by_fact",
        memory.indexes.availability_since_by_fact,
    );
    print_entry(
        "index.relations_by_source_kind",
        memory.indexes.relations_by_source_kind,
    );

    let query_type = handle.platform_type_by_id("platform_type:Запрос");
    let filter_type = handle.platform_type_by_id("platform_type:ОтборКомпоновкиДанных");
    let query_table_with_field = handle
        .query_tables_by_identifier("Справочник")
        .into_iter()
        .next()
        .or_else(|| {
            handle
                .query_tables_by_identifier("ОсновнаяТаблица")
                .into_iter()
                .next()
        });
    let query_table_with_parameter = handle
        .query_tables_by_identifier("ЗадачаТаблицаЗадачПоИсполнителю")
        .into_iter()
        .next();
    let query_field_with_type = handle
        .query_tables_by_identifier("БизнесПроцесс")
        .into_iter()
        .next()
        .and_then(|table| {
            handle
                .query_fields_by_name(table, "Ссылка")
                .into_iter()
                .next()
        });

    measure("exact_fact_id", iterations, || {
        handle.facts_by_id("platform_type:Запрос").len()
    });
    measure("type_by_name", iterations, || {
        handle.platform_types_by_name("Запрос").len()
    });
    measure("type_template_by_key", iterations, || {
        handle
            .platform_types_by_template_key("Catalog", "Manager")
            .len()
    });
    measure("members_by_owner", iterations, || {
        query_type.map_or(0, |owner| handle.members_of_type(owner).len())
    });
    measure("member_by_owner_name_kind", iterations, || {
        query_type.map_or(0, |owner| {
            handle
                .member_by_owner_name_kind(owner, "Текст", Some(HbkTypeMemberKind::Property))
                .len()
        })
    });
    measure("callable_by_owner_name", iterations, || {
        query_type.map_or(0, |owner| {
            handle.callable_by_owner_name(owner, "Выполнить").len()
        })
    });
    measure("constructors_by_type", iterations, || {
        query_type.map_or(0, |owner| handle.constructors_of_type(owner).len())
    });
    measure("global_by_domain_name_kind", iterations, || {
        handle
            .globals_by_domain_name_kind(
                HbkLanguageDomain::Bsl,
                "Сообщить",
                Some(HbkGlobalFactKind::Method),
            )
            .len()
    });
    measure("module_context_by_kind", iterations, || {
        handle
            .module_context_events(HbkLanguageDomain::Bsl, "bsl", "managed_application")
            .len()
    });
    measure("query_table_by_name", iterations, || {
        handle.query_tables_by_name("Таблица справочника").len()
    });
    measure("query_field_by_table_name", iterations, || {
        query_table_with_field.map_or(0, |table| {
            handle.query_fields_by_name(table, "Ссылка").len()
        })
    });
    measure("query_param_by_table_name", iterations, || {
        query_table_with_parameter.map_or(0, |table| {
            handle.query_parameters_by_name(table, "Исполнитель").len()
        })
    });
    measure("availability_by_fact", iterations, || {
        filter_type.map_or(0, |fact| {
            let fact = HbkFactRef::PlatformType(fact);
            handle.availability_contexts(fact).len()
                + usize::from(handle.available_since(fact).is_some())
        })
    });
    measure("relation_by_source_kind", iterations, || {
        query_field_with_type.map_or(0, |field| {
            handle
                .relations_by_source_kind(HbkFactRef::QueryField(field), "has_type")
                .len()
        })
    });

    Ok(())
}

fn print_entry(name: &str, entry: HbkFactSnapshotMemoryEntry) {
    println!("{name}.count={}", entry.count);
    println!("{name}.bytes={}", entry.bytes);
}

fn print_duration(name: &str, duration: Duration) {
    println!("{name}_ms={}", duration.as_millis());
    println!("{name}_ns={}", duration.as_nanos());
}

fn measure(name: &str, iterations: usize, mut operation: impl FnMut() -> usize) {
    let start = Instant::now();
    let mut total = 0usize;
    for _ in 0..iterations {
        total = total.wrapping_add(black_box(operation()));
    }
    let elapsed = start.elapsed();
    println!("{name}.iterations={iterations}");
    println!("{name}.total_ns={}", elapsed.as_nanos());
    println!(
        "{name}.avg_ns={}",
        elapsed.as_nanos() / iterations.max(1) as u128
    );
    println!("{name}.observed_total={total}");
}
