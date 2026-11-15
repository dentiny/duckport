"""Exercise arXiv S3 billing opt-in using the upstream manifest shape and local TAR fixture."""
import os

from service_fixture import REPO, fixture_server, literal, run_sql_tests

if __name__ == "__main__":
    with fixture_server() as (endpoint, routes, requests):
        for name in ["arXiv_src_manifest.xml", "arXiv_src_2410_001.tar"]:
            routes[f"/arxiv-test/src/{name}"] = (200, (REPO / "testdata/arxiv/src" / name).read_bytes())
        httpfs = os.environ.get("HTTPFS_EXTENSION", "httpfs")
        setup = f"""LOAD {literal(httpfs)};
CREATE SECRET arxiv_test (TYPE s3, KEY_ID 'fixture-key', SECRET 'fixture-secret',
    REGION 'us-east-1', ENDPOINT {literal(endpoint)}, URL_STYLE 'path',
    USE_SSL false, SCOPE 's3://arxiv-test/');"""
        count = run_sql_tests(REPO / "test/integration/sql/arxiv_s3.test", {}, setup)
        assert requests, "S3 reads were not exercised"
        assert all(headers.get("x-amz-request-payer") == "requester" for _, headers in requests), requests
        assert all("AWS4-HMAC-SHA256" in headers.get("authorization", "") for _, headers in requests), requests
        requests.clear()
        count += run_sql_tests(REPO / "test/integration/sql/arxiv_s3_default.test", {}, setup)
        assert requests
        assert all("x-amz-request-payer" not in headers for _, headers in requests), requests
        print(f"arxiv: {count} S3 SQL checks passed; signed requester-pays headers verified")
