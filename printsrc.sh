find . -type f -name "*.rs" -print0 | while IFS= read -r -d '' file; do
  echo "===== $file ====="
  cat "$file"
  echo ""
done