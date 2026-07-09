window.BENCHMARK_DATA = {
  "lastUpdate": 1783630128601,
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
      },
      {
        "commit": {
          "author": {
            "email": "burravlev@icloud.com",
            "name": "Daniil Buravlev",
            "username": "daniilburavlev"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "475f6beea4466b3e26f2111412152bbd270556eb",
          "message": "Add benchmarks checks (#6)\n\n* Finish network server layer\n\n* Fix network, fix bench action\n\n* Fix CI bench",
          "timestamp": "2026-07-09T23:47:17+03:00",
          "tree_id": "57cd0f3bff099c2b37dfe5a9e025970423445fcd",
          "url": "https://github.com/daniilburavlev/simpledb/commit/475f6beea4466b3e26f2111412152bbd270556eb"
        },
        "date": 1783630128032,
        "tool": "cargo",
        "benches": [
          {
            "name": "insert",
            "value": 251931,
            "range": "± 37637",
            "unit": "ns/iter"
          },
          {
            "name": "join",
            "value": 10493719,
            "range": "± 148265",
            "unit": "ns/iter"
          },
          {
            "name": "index_insert",
            "value": 275205,
            "range": "± 35376",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}