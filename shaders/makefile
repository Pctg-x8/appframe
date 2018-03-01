# Makefile for GLSL Shader Sources from interlude

CC_GLSL = glslc
.SUFFIXES: .vert .frag .geom .vso .fso .gso
VERT_OUTS = $(patsubst %.vert,%.vso,$(shell find . -type f -name '*.vert'))
FRAG_OUTS = $(patsubst %.frag,%.fso,$(shell find . -type f -name '*.frag'))
GEOM_OUTS = $(patsubst %.geom,%.gso,$(shell find . -type f -name '*.geom'))
ALL_OUTS = $(VERT_OUTS) $(FRAG_OUTS) $(GEOM_OUTS)

all: $(ALL_OUTS)

clean:
	rm -r **/*.spv

.vert.vso:
	$(CC_GLSL) $< -o $@

.frag.fso:
	$(CC_GLSL) $< -o $@

.geom.spv:
	$(CC_GLSL) $< -o $@
