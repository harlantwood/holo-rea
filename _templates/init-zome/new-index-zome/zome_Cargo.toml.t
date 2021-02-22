---
to: <%=dna_path%>/zomes/<%= h.changeCase.snake(foreign_record_name) %>_idx/code/Cargo.toml
---
[package]
name = "hc_zome_<%= h.changeCase.snake(foreign_record_name) %>_index_<%= h.changeCase.snake(local_dna_name) %>"
version = "0.1.0"
authors = ["<%=package_author_name%> <<%=package_author_email%>>"]
edition = "2018"

[dependencies]
serde = "1"
# :DUPE: hdk-rust-revid
hdk3 = {git = "https://github.com/holochain/holochain", rev = "5f1d6f410", package = "hdk"}

hdk_graph_helpers = { path = "../../../../../lib/hdk_graph_helpers" }
vf_core = { path = "../../../../../lib/vf_core" }
hc_zome_rea_<%= h.changeCase.snake(local_record_name) %>_storage_consts = { path = "../../../../../lib/rea_<%= h.changeCase.snake(local_record_name) %>/storage_consts" }
hc_zome_rea_<%= h.changeCase.snake(foreign_record_name) %>_storage_consts = { path = "../../../../../lib/rea_<%= h.changeCase.snake(foreign_record_name) %>/storage_consts" }

[lib]
path = "src/lib.rs"
crate-type = ["cdylib"]
