window.BENCHMARK_DATA = {
  "lastUpdate": 1783166302687,
  "repoUrl": "https://github.com/daniilburavlev/simpledb",
  "entries": {
    "Benchmark": [
      {
        "commit": {
          "author": {
            "name": "daniilburavlev",
            "username": "daniilburavlev"
          },
          "committer": {
            "name": "daniilburavlev",
            "username": "daniilburavlev"
          },
          "id": "6026536cffa4b6f6621b57151a9520a2c3729af3",
          "message": "Add benchmarks checks",
          "timestamp": "2026-07-04T10:28:16Z",
          "url": "https://github.com/daniilburavlev/simpledb/pull/6/commits/6026536cffa4b6f6621b57151a9520a2c3729af3"
        },
        "date": 1783166302462,
        "tool": "cargo",
        "benches": [
          {
            "name": "insert",
            "value": 227231,
            "range": "± 34969",
            "unit": "ns/iter"
          },
          {
            "name": "join",
            "value": 12596618,
            "range": "± 169903",
            "unit": "ns/iter"
          },
          {
            "name": "index_insert",
            "value": 248303,
            "range": "± 32725",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}