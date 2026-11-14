from service_fixture import run_case

if __name__ == "__main__":
    run_case("zenodo", authenticated=False)
    run_case("zenodo")
