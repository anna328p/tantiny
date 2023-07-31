use std::collections::HashMap;
use std::str::FromStr;
use rutie::{methods, Object, AnyObject, Integer, NilClass, Array, RString, Hash, class, VerifiedObject, Class, TryConvert};
use tantivy::{doc, Document, Term, ReloadPolicy, Index, IndexWriter, IndexReader, DateTime};
use tantivy::schema::{Schema, TextOptions, TextFieldIndexing, IndexRecordOption, FacetOptions, IntOptions, Cardinality, STRING, STORED, INDEXED, FAST};
use tantivy::collector::TopDocs;
use tantivy::directory::MmapDirectory;

use crate::helpers::{scaffold, try_unwrap_params, TryUnwrap};
use crate::query::{unwrap_query, RTantinyQuery};
use crate::tokenizer::{unwrap_tokenizer, RTantinyTokenizer};

pub struct TantinyIndex {
    pub(crate) schema: Schema,
    pub(crate) index: Index,
    pub(crate) index_writer: Option<IndexWriter>,
    pub(crate) index_reader: IndexReader,
}

scaffold!(RTantinyIndex, TantinyIndex, "Index");

pub(crate) fn unwrap_index(index: &RTantinyIndex) -> &TantinyIndex {
    index.get_data(&*TANTINY_INDEX_WRAPPER)
}

pub(crate) fn unwrap_index_mut(index: &mut RTantinyIndex) -> &mut TantinyIndex {
    index.get_data_mut(&*TANTINY_INDEX_WRAPPER)
}

class!(RTantinySchemaField);

impl VerifiedObject for RTantinySchemaField {
    fn is_correct_type<T: Object>(object: &T) -> bool {
        let field_class = Class::from_existing("::Tantiny::Schema::Field");
        let ancestors = field_class.ancestors();

        ancestors.iter().any(|&c| c == field_class)
    }

    fn error_message() -> &'static str {
        "Error converting to Field"
    }
}

impl RTantinySchemaField {
    fn field_type(self) -> String {
        unsafe { self.send("type", &[]) }
            .try_unwrap()
    }

    fn key(self) -> String {
        unsafe { self.send("key", &[]) }
            .try_unwrap()
    }

    fn stored(self) -> bool {
        unsafe { self.send("stored", &[]) }
            .try_unwrap()
    }

    fn tokenizer(self) -> Option<String> {
        let val = unsafe { self.send("tokenizer", &[]) };

        match RString::try_convert(val) {
            Ok(s) => Some(s.to_string()),
            Err(_) => None
        }
    }
}

impl TryUnwrap<RTantinySchemaField> for AnyObject {
    fn try_unwrap(self) -> RTantinySchemaField {
        self.try_convert_to::<RTantinySchemaField>().unwrap()
    }
}

class!(RTantinySchema);

impl VerifiedObject for RTantinySchema {
    fn is_correct_type<T: Object>(object: &T) -> bool {
        let field_class = Class::from_existing("::Tantiny::Schema");
        let ancestors = field_class.ancestors();

        ancestors.iter().any(|&c| c == field_class)
    }

    fn error_message() -> &'static str {
        "Error converting to Schema"
    }
}

impl RTantinySchema {
    fn fields(self) -> HashMap<String, RTantinySchemaField> {
        unsafe { self.send("fields", &[]) }
            .try_convert_to::<Hash>()
            .unwrap()
            .try_unwrap()
    }

    fn default_tokenizer(self) -> String {
        unsafe { self.send("default_tokenizer", &[]) }
            .try_unwrap()
    }
}

impl TryUnwrap<RTantinySchema> for AnyObject {
    fn try_unwrap(self) -> RTantinySchema {
        self.try_convert_to::<RTantinySchema>().unwrap()
    }
}

#[rustfmt::skip::macros(methods)]
methods!(
    RTantinyIndex,
    _itself,

    fn new_index(
        path: RString,
        schema: RTantinySchema
    ) -> RTantinyIndex {
        try_unwrap_params!(
            path: String
        );

        let index_path = MmapDirectory::open(path).try_unwrap();
        let mut schema_builder = Schema::builder();

        for (name, field) in schema.unwrap().fields() {
            let field_type = field.field_type().as_str();

            let stored = field.stored();

            let int_options = IntOptions {
                indexed: true,
                stored: stored,
                fast: Some(Cardinality::SingleValue),
            };

            if field_type == "text" {
                let tokenizer_name = match field.tokenizer() {
                    Some(s) => &*s,
                    None => "default"
                };

                let indexing = TextFieldIndexing::default()
                    .set_tokenizer(tokenizer_name)
                    .set_index_option(IndexRecordOption::WithFreqsAndPositions);

                let options = TextOptions {
                    indexing: Some(indexing),
                    stored: stored
                };

                schema_builder.add_text_field(&*name, options);

            } else if field_type == "string" {
                let options =
                    TextOptions { stored: stored, indexing: None }
                        | STRING;

                schema_builder.add_text_field(&*name, options);

            } else if field_type == "integer" {
                schema_builder.add_i64_field(&*name, int_options);
            } else if field_type == "double" {
                schema_builder.add_f64_field(&*name, int_options);
            } else if field_type == "date" {
                schema_builder.add_date_field(&*name, int_options);
            } else if field_type == "facet" {
                let options = FacetOptions {
                    indexed: true,
                    stored: true
                };

                schema_builder.add_text_field(&*name, options);
            }
        }

        schema_builder.add_text_field("id", STRING | STORED);

        let schema = schema_builder.build();
        let index = Index::open_or_create(index_path, schema.clone()).try_unwrap();
        let tokenizers = index.tokenizers();

        tokenizers.register("default", unwrap_tokenizer(&default_tokenizer).clone());

        for (field, tokenizer) in field_tokenizers {
            tokenizers.register(&field, unwrap_tokenizer(&tokenizer).clone())
        }

        let index_writer = None;

        let index_reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()
            .try_unwrap();
        
        klass().wrap_data(
            TantinyIndex { index, index_writer, index_reader, schema },
            &*TANTINY_INDEX_WRAPPER
        )
    }

    fn add_document(
        id: RString,
        text_fields: Hash,
        string_fields: Hash,
        integer_fields: Hash,
        double_fields: Hash,
        date_fields: Hash,
        facet_fields: Hash
    ) -> NilClass {
        try_unwrap_params!(
            id: String,
            text_fields: HashMap<String, String>,
            string_fields: HashMap<String, String>,
            integer_fields: HashMap<String, i64>,
            double_fields: HashMap<String, f64>,
            date_fields: HashMap<String, String>,
            facet_fields: HashMap<String, String>
        );

        let internal = unwrap_index(&_itself);
        let index_writer = internal.index_writer.as_ref().try_unwrap();
        let schema = &internal.schema;

        let mut doc = Document::default();

        let id_field = schema.get_field("id").try_unwrap();
        doc.add_text(id_field, &id);

        for (key, value) in text_fields.iter() {
            let field = schema.get_field(key).try_unwrap();
            doc.add_text(field, value);
        }

        for (key, value) in string_fields.iter() {
            let field = schema.get_field(key).try_unwrap();
            doc.add_text(field, value);
        }

        for (key, &value) in integer_fields.iter() {
            let field = schema.get_field(key).try_unwrap();
            doc.add_i64(field, value);
        }

        for (key, &value) in double_fields.iter() {
            let field = schema.get_field(key).try_unwrap();
            doc.add_f64(field, value);
        }

        for (key, value) in date_fields.iter() {
            let field = schema.get_field(key).try_unwrap();
            let value = DateTime::from_str(value).try_unwrap();
            doc.add_date(field, &value);
        }

        for (key, value) in facet_fields.iter() {
            let field = schema.get_field(key).try_unwrap();
            doc.add_facet(field, &value);
        }

        let doc_id = Term::from_field_text(id_field, &id);
        index_writer.delete_term(doc_id.clone());

        index_writer.add_document(doc);

        NilClass::new()
    }

    fn delete_document(id: RString) -> NilClass {
        try_unwrap_params!(id: String);

        let internal = unwrap_index(&_itself);
        let index_writer = internal.index_writer.as_ref().unwrap();

        let id_field = internal.schema.get_field("id").try_unwrap();
        let doc_id = Term::from_field_text(id_field, &id);

        index_writer.delete_term(doc_id.clone());

        NilClass::new()
    }

    fn acquire_index_writer(
        overall_memory: Integer
    ) -> NilClass {
        try_unwrap_params!(overall_memory: i64);

        let internal = unwrap_index_mut(&mut _itself);

        let mut index_writer = internal.index
            .writer(overall_memory as usize)
            .try_unwrap();

        internal.index_writer = Some(index_writer);

        NilClass::new()
    }

    fn release_index_writer() -> NilClass {
        let internal = unwrap_index_mut(&mut _itself);

        drop(internal.index_writer.as_ref().try_unwrap());
        internal.index_writer = None;

        NilClass::new()
    }

    fn commit() -> NilClass {
        let internal = unwrap_index_mut(&mut _itself);
        let index_writer = internal.index_writer.as_mut().try_unwrap();

        index_writer.commit().try_unwrap();

        NilClass::new()
    }

    fn reload() -> NilClass {
        unwrap_index(&_itself).index_reader.reload().try_unwrap();

        NilClass::new()
    }

    fn search(
        query: AnyObject,
        limit: Integer
    ) -> Array {
        try_unwrap_params!(
            query: RTantinyQuery,
            limit: i64
        );

        let internal = unwrap_index(&_itself);
        let id_field = internal.schema.get_field("id").try_unwrap();
        let searcher = internal.index_reader.searcher();
        let query = unwrap_query(&query);

        let top_docs = searcher
            .search(query, &TopDocs::with_limit(limit as usize))
            .try_unwrap();

        let mut array = Array::with_capacity(top_docs.len());

        for (_score, doc_address) in top_docs {
            let doc = searcher.doc(doc_address).try_unwrap();
            if let Some(value) = doc.get_first(id_field) {
                if let Some(id) = (&*value).text() {
                    array.push(RString::from(String::from(id)));
                }
            }
        }

        array
    }
);

pub(super) fn init() {
    klass().define(|klass| {
        klass.def_self("__new", new_index);
        klass.def("__add_document", add_document);
        klass.def("__delete_document", delete_document);
        klass.def("__acquire_index_writer", acquire_index_writer);
        klass.def("__release_index_writer", release_index_writer);
        klass.def("__commit", commit);
        klass.def("__reload", reload);
        klass.def("__search", search);
    });
} 
