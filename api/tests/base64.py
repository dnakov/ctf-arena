import sys, base64
print(base64.b64decode(sys.stdin.read().strip()).decode(), end="")
