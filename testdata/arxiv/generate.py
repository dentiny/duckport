"""Regenerate synthetic PDF and metadata payloads with Python and an installed DuckDB CLI."""
from io import BytesIO
from pathlib import Path
import subprocess
import tarfile

root = Path(__file__).resolve().parent
pdf = root / 'pdf'
pdf.mkdir(exist_ok=True)
archive = pdf / 'arXiv_pdf_2410_001.tar'
entries = [('2410/2410.01234v2.pdf', b'%PDF-first'), ('2410/2410.01235.pdf', b'%PDF-second')]
with tarfile.open(archive, 'w', format=tarfile.USTAR_FORMAT) as tar:
    for name, data in entries:
        info = tarfile.TarInfo(name)
        info.size = len(data)
        info.mode = 0o644
        info.mtime = 0
        tar.addfile(info, BytesIO(data))
(pdf / 'arXiv_pdf_manifest.xml').write_text(
    '<arXivPDF><file><filename>pdf/arXiv_pdf_2410_001.tar</filename>'
    '<md5sum>00000000000000000000000000000000</md5sum><num_items>2</num_items>'
    f'<seq_num>1</seq_num><size>{archive.stat().st_size}</size>'
    '<timestamp>2024-10-31 00:00:00</timestamp><yymm>2410</yymm></file></arXivPDF>\n'
)
snapshot = str(root / 'metadata.parquet').replace("'", "''")
subprocess.run(['duckdb', '-c', f"""
COPY (SELECT * FROM (VALUES
    ('oai:arXiv.org:2410.01234', 'First', ['Ada'], {{'category': 'cs.AI'}}, DATE '2024-10-01'),
    ('2410.01235', 'Second', ['Grace', 'Lin'], {{'category': 'cs.LG'}}, DATE '2024-10-02')
) AS records(identifier, title, authors, attributes, submitted))
TO '{snapshot}' (FORMAT PARQUET);
"""], check=True)
