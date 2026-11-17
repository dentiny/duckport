#include "error.hpp"
#include "source/source_factory.hpp"

namespace duckdb {

SourceFactory::SourceFactory(SourcePtr source_p) : source(std::move(source_p)) {
}

void SourceFactory::GetSchema(ArrowSchema &schema) {
	CheckError(duckport_source_get_schema(source.get(), &schema));
}

unique_ptr<ArrowArrayStreamWrapper> SourceFactory::ProduceStream(ArrowStreamParameters &parameters) {
	auto stream = make_uniq<ArrowArrayStreamWrapper>();
	vector<const char *> columns;
	columns.reserve(parameters.projected_columns.columns.size());
	for (const auto &column : parameters.projected_columns.columns) {
		columns.push_back(column.c_str());
	}
	CheckError(duckport_source_get_stream(source.get(), columns.data(), columns.size(), &stream->arrow_array_stream));
	return stream;
}

} // namespace duckdb
