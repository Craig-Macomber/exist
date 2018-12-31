# exist
Self describing persistence library in Rust.

Have some data, and also some type, and want to existify something as close to that type as possible from the data with proper semantics? If thats what you want, some day this library might help you do this, as well as export said data in the first place.

This library is very unfinished and it not currently usable at all. It will hopefully evolve into a usable proof of concept, and then maybe get productionized, however my focus is currently on learning rust, and experimenting with various rust features, tools and design patterns.

The goal of exist is to provide an easy way to serialize rust data structures (mainly tree like ones consisting of structs and sequence types) such that they can deserialized and interpreted even if the schema (the rust types you are deserializing into) do no match the ones that were serialized. This requires the data to be self describing, and the deserializer must handle missing data, extra data, and otherwise out of schema data.

exist is attempting to solve this problem while optimizing for the following properties:
- easy to use reader interface for deserializer: the deserialized data may be arbitrarily far out of schema which can be verbose and error prone for users to process. Exist will use declaritive annotations on the schema types provide common patterns for how and where to handle out of schema data. Ex: for each field you choose how to handel if it is missing or invalid. You can provide a default (for just the missing case, or for missing or invalid), expose it as an enum (union of valid data and a reflection like interface for out of schema data if you want to look at it), or mark the parent struct (containing the field) as invalid and resolve it using its policy. This will be accomplished by generating a reading interface via macros. Status: Not started.
- compact: when most of the data is in structs that occurs many times, the serialized size will approach the in memory size since the type metadata will be deduplicated, and the data layout will roughly match the in memory layout. This is one area exist is specifically trying to outperform alternatives like xml, json, and protobuf with optional fields.
- does not require centralized schema management: nothing will break if multiple groups fork and independently extend and edit schema types. Fields and types are identifies by id numbers, but unlike protobuf tags, compactness does not require choosing small values. Id's are 128 bits, and thus are a super-set of UUIDs, and thus random UUIDs can be used to avoid the risk of id collisions. Intentional collisions and/or malicious documents with collisions will not break anything either.
- efficient encode and decode: the type information from the schema types will be used to generate encoding templates. This serves as a compression schema that deduplicates the type metadata without having to actually look for redundancies to compress. It also enables picking a compressed for for structs that matches their in memory layout to enable a fast path encode and decode if the schema types match, while still allowing all documents to be decoded in the mismatch case. This will zero copy decode in many cases.

Non goals:
- human readable format: if you want this, use something like json or xml
- good compression of data: exist only tries to compress the type meta-data needed for self description. It could be extended to do data compression, or the output could be run through a general purpose compression, however these operations are out of scop of the goals of this project

Potential future features:
- extend reader interface to support edits and re-export to enable persisting data the current application did not need or understand when performing a read edit overwrite process (ex: updating a document).
- extensible contract system: support detecting of data as invalid even if the types match. This would allow using the out of schema data support to handel violations of any checkable constraint. This could be a useful standalone library, but needs to be built in a layerable way so it can compose with the type based part.
- format converters/updaters: allow providing the reader with some alternative formats (which it could expose via enum) and also optional converters which can allow handling alternative formats without having to expose them in the reader interface
