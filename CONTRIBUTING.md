# How to add new assistant features

This assumes you're in the project root directory, and that the features being
added are completely independent of other, pre-existing features.

1. Start a Postgres instance.

   ```bash
   docker compose up -d db
   ```

2. Create a migration and add its SQL up/down statements.

   ```bash
   diesel migration generate <MIGRATION_NAME>
   ```
   
   (Ex: [git show ec443a37bc243a37c5a7efd1412b33693c6ada6b][0])

3. Update the Rust schema.

   ```bash
   diesel database reset --database-url postgres://self:hosted@localhost:5432/toi \
    --migration-dir toi_server/migrations
   ```

   (Ex: [git show f4b01c7a6eea0ae73f1d577908d3220a24001d33][1])

4. Add data structures/models.

   (Ex: [git show ddd0687d95df929539f60e6732963236f8314c03][2])

5. Add endpoints.

   (Ex: [git show 96a9633b4b47492e6cd97fddb3a372410d9e42f7][3])

6. Add endpoints to the main router.

   (Ex: [git show 4f1b50d58fef5e450428cbd54a8fd78c67c646af][4])

7. Add tests.

   (Ex: [git show 088433e17fa2a431e7ea246b3820fc0772e64577][5])

8. Test everything.

   ```bash
   f=docker-compose.test.yaml \
      && docker compose -f $f up -d --build \
      && docker compose -f $f logs -f api \
      && docker compose -f $f down
   ```

   The logs should show all tests passing.

9. Make final formatting and clippy updates.

   ```bash
   cargo fmt
   cargo clippy
   ```

[0]: https://github.com/theOGognf/toi/commit/ec443a37bc243a37c5a7efd1412b33693c6ada6b
[1]: https://github.com/theOGognf/toi/commit/f4b01c7a6eea0ae73f1d577908d3220a24001d33
[2]: https://github.com/theOGognf/toi/commit/ddd0687d95df929539f60e6732963236f8314c03
[3]: https://github.com/theOGognf/toi/commit/96a9633b4b47492e6cd97fddb3a372410d9e42f7
[4]: https://github.com/theOGognf/toi/commit/4f1b50d58fef5e450428cbd54a8fd78c67c646af
[5]: https://github.com/theOGognf/toi/commit/088433e17fa2a431e7ea246b3820fc0772e64577
