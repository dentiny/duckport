from service_fixture import run_case

if __name__ == "__main__":
    run_case("dataverse", authenticated=False)
    run_case("dataverse")
