# Fix a maintainer's author email

<div class="cookbook-meta">
  <span class="level-badge level-easy">Easy</span>
  <code>Repo: any</code>
  <code>Size: small</code>
</div>

A maintainer used the wrong email address and wants to correct it across history.

## Goal

Rewrite author/committer email for a single person.

## Steps

```sh
gk rewrite \
  -n "Maintainer Name" \
  -e "maintainer@example.test" \
  -o "old-maintainer@example.test" \
  -O "Maintainer Name"
```

## Notes

- `-O` ensures only commits by that name are rewritten.
- This updates both author and committer fields.
