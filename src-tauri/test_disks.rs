use sysinfo::Disks;
fn main() {
    let disks = Disks::new_with_refreshed_list();
    for d in disks.list() {
        println!("mount: {:?}, fs: {:?}, total: {}, free: {}", d.mount_point(), d.file_system(), d.total_space(), d.available_space());
    }
}
