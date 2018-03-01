#!/bin/sh

# For Building on Darwin Platform(macOS)

cargo build --example with_ferrite --features with_ferrite,ferrite/VK_MVK_macos_surface
install_name_tool -change @rpath/vulkan.framework/Versions/A/vulkan @executable_path/vulkan.framework/Versions/A/vulkan target/debug/examples/with_ferrite
cp -r $VK_SDK_PATH/macOS/Frameworks/vulkan.framework target/debug/examples/
target/debug/examples/with_ferrite
