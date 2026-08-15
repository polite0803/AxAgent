import psycopg2

conn = psycopg2.connect(host="localhost", port=5432, dbname="axinvest", user="postgres", password="Hjdssyqsyl410")
conn.autocommit = True
cur = conn.cursor()

print("=== conversations columns ===")
cur.execute("""
SELECT column_name, data_type
FROM information_schema.columns
WHERE table_schema='public' AND table_name='conversations'
ORDER BY ordinal_position
""")
for r in cur.fetchall():
    print(r)

print("\n=== triggers on conversations ===")
cur.execute("""
SELECT tgname, tgtype, pg_get_triggerdef(t.oid)
FROM pg_trigger t JOIN pg_class c ON t.tgrelid=c.oid
WHERE c.relname='conversations' AND NOT t.tgisinternal
""")
rows = cur.fetchall()
print(rows if rows else "none")

print("\n=== sample conversation id / message_count types ===")
cur.execute("SELECT id, message_count, updated_at FROM conversations LIMIT 2")
for r in cur.fetchall():
    print(r)

print("\n=== test: increment with $1 placeholders (correct for PG) ===")
try:
    cur.execute("UPDATE conversations SET message_count = message_count + 1, updated_at = %s WHERE id = %s", (1234567890, "x-nonexistent"))
    print("OK rows:", cur.rowcount)
except Exception as e:
    print("ERR:", e)

cur.close()
conn.close()