QEMU_PLUGIN_CFLAGS := $(shell pkg-config --cflags glib-2.0)
CFLAGS := -fPIC -Wall -O2 -I. $(QEMU_PLUGIN_CFLAGS)

sandbox.so: sandbox.c
	$(CC) $(CFLAGS) -shared -o $@ $<

clean:
	rm -f sandbox.so
