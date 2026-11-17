# ATIF fixtures

Copied from open-arrow-access's MIT-licensed ATIF fixtures. `trajectory.json` contains one trajectory; `trajectories.jsonl` contains three, with blank lines and CRLF. They include Unicode messages and tool calls. The bad inputs contain a truncated second record or unsupported schema version.

`media.json` adds sink-ingestion coverage for a relative image reference, inline
audio data, and content text. The image points to the existing Zenodo attachment
fixture; media bytes are not interpreted as image/audio codecs by the sink.
