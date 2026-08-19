import sys
pw = open(r'C:\Users\polit\.axagent\.pgprobe_pw.txt').read().strip()
import psycopg2
conn = psycopg2.connect(host='localhost', port=5432, dbname='axinvest', user='postgres', password=pw)
cur = conn.cursor()
cur.execute('select COALESCE(max(version),0) from axagent_schema_version')
print('schema_version_max', cur.fetchone()[0])
cur.execute("""
  select p.name, p.enabled, count(*) as n_models,
         count(*) filter (where m.enabled = 1) as n_enabled_models
  from providers p left join models m on m.provider_id = p.id
  group by p.id order by p.sort_order
""")
print('name | prov_enabled | n_models | n_enabled_models')
for r in cur.fetchall():
    print(r)
conn.close()