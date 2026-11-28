//! Check the Android window against the renderer's requirements before device creation.

use std::ffi::CStr;

use ash::{Entry, Instance, khr, vk};

type Result<T> = std::result::Result<T, String>;

fn has_extension(available: &[vk::ExtensionProperties], name: &CStr) -> bool {
    available
        .iter()
        .any(|ext| ext.extension_name_as_c_str() == Ok(name))
}

/// The caller must own a live ANativeWindow for this entire call.
pub unsafe fn probe(window: *mut vk::ANativeWindow) -> Result<String> {
    let entry = unsafe { Entry::load() }.map_err(|err| err.to_string())?;
    let version = unsafe { entry.try_enumerate_instance_version() }
        .map_err(|err| err.to_string())?
        .unwrap_or(vk::API_VERSION_1_0);
    if version < vk::API_VERSION_1_3 {
        return Err("loader requires Vulkan 1.3".into());
    }
    let extensions = unsafe { entry.enumerate_instance_extension_properties(None) }
        .map_err(|err| err.to_string())?;
    let required = [khr::surface::NAME, khr::android_surface::NAME];
    for name in required {
        if !has_extension(&extensions, name) {
            return Err(format!("missing {}", name.to_string_lossy()));
        }
    }
    let names = required.map(CStr::as_ptr);
    let app = vk::ApplicationInfo::default()
        .application_name(c"RUSerBox")
        .api_version(vk::API_VERSION_1_3);
    let info = vk::InstanceCreateInfo::default()
        .application_info(&app)
        .enabled_extension_names(&names);
    let instance = unsafe { entry.create_instance(&info, None) }.map_err(|err| err.to_string())?;
    let android = khr::android_surface::Instance::new(&entry, &instance);
    let surface_api = khr::surface::Instance::new(&entry, &instance);
    let info = vk::AndroidSurfaceCreateInfoKHR::default().window(window);
    let result = unsafe { android.create_android_surface(&info, None) }
        .map_err(|err| err.to_string())
        .and_then(|surface| {
            let result = inspect(&instance, &surface_api, surface);
            unsafe { surface_api.destroy_surface(surface, None) };
            result
        });
    unsafe { instance.destroy_instance(None) };
    result
}

fn inspect(
    instance: &Instance,
    api: &khr::surface::Instance,
    surface: vk::SurfaceKHR,
) -> Result<String> {
    let devices =
        unsafe { instance.enumerate_physical_devices() }.map_err(|err| err.to_string())?;
    for device in devices {
        let props = unsafe { instance.get_physical_device_properties(device) };
        if props.api_version < vk::API_VERSION_1_3 {
            continue;
        }
        let mut dynamic = vk::PhysicalDeviceDynamicRenderingFeatures::default();
        let mut features = vk::PhysicalDeviceFeatures2::default().push_next(&mut dynamic);
        unsafe { instance.get_physical_device_features2(device, &mut features) };
        let extensions = unsafe { instance.enumerate_device_extension_properties(device) }
            .map_err(|err| err.to_string())?;
        if dynamic.dynamic_rendering == vk::FALSE
            || !has_extension(&extensions, khr::swapchain::NAME)
        {
            continue;
        }
        let queues = unsafe { instance.get_physical_device_queue_family_properties(device) };
        for (index, queue) in queues.iter().enumerate() {
            if queue.queue_count == 0
                || !queue
                    .queue_flags
                    .contains(vk::QueueFlags::GRAPHICS | vk::QueueFlags::COMPUTE)
            {
                continue;
            }
            let present =
                unsafe { api.get_physical_device_surface_support(device, index as u32, surface) }
                    .map_err(|err| err.to_string())?;
            if !present {
                continue;
            }
            let formats = unsafe { api.get_physical_device_surface_formats(device, surface) }
                .map_err(|err| err.to_string())?;
            let modes = unsafe { api.get_physical_device_surface_present_modes(device, surface) }
                .map_err(|err| err.to_string())?;
            let caps = unsafe { api.get_physical_device_surface_capabilities(device, surface) }
                .map_err(|err| err.to_string())?;
            if !formats.is_empty()
                && modes.contains(&vk::PresentModeKHR::FIFO)
                && caps
                    .supported_usage_flags
                    .contains(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            {
                let name = props
                    .device_name_as_c_str()
                    .map_err(|err| err.to_string())?;
                return Ok(format!(
                    "{} (queue family {index}, {} surface formats)",
                    name.to_string_lossy(),
                    formats.len()
                ));
            }
        }
    }
    Err("no Vulkan 1.3 device with dynamic rendering, a presentable graphics/compute queue and a FIFO color attachment".into())
}
