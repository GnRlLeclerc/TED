//! Filesystem recursion utilities

use super::item::Item;
use ted_config::Config;
use ted_fs::{Filesystem, Folder, FolderKey};

/// Recursively count the maximum number of visible items
/// when displaying all folders and their children (if open)
pub fn count_items(fs: &Filesystem, folder: &Folder, config: &Config) -> usize {
    let mut count = folder.child_files.len();

    for folder_key in &folder.child_folders {
        let folder = fs.folder(*folder_key);
        if folder.open {
            count += count_items(fs, folder, config) + 1;
        } else if !folder.hidden(config) {
            count += 1;
        }
    }

    count
}

/// Recursively collect items to display, with the given skip and take params.
pub fn collect_items(
    fs: &Filesystem,
    folder: &Folder,
    config: &Config,
    items: &mut Vec<(Item, usize)>,
    count: &mut usize,
    depth: usize,
    skip: usize,
    take: usize,
) {
    // *************************************** //
    //       Iterate through subfolders        //
    // *************************************** //

    for key in &folder.child_folders {
        // Check if we have already taken enough lines
        if *count >= skip + take {
            return;
        }

        let folder = &fs.folder(*key);

        // Skip the folder if it is hidden and not open
        if folder.hidden(config) && !folder.open {
            continue;
        }

        if *count >= skip {
            items.push((key.into(), depth));
        }

        *count += 1;

        if folder.open {
            collect_items(fs, folder, config, items, count, depth + 1, skip, take);
        }
    }

    // *************************************** //
    //       Iterate through child files       //
    // *************************************** //

    let files = &folder.child_files;

    // Skip the files if they are before the skip index
    if *count + files.len() < skip {
        *count += files.len();
        return;
    }

    // Take the files if they are within the take range
    let start = skip.saturating_sub(*count);
    let end = (skip + take).saturating_sub(*count).min(files.len());
    *count += files.len();

    for key in &files[start..end] {
        items.push((key.into(), depth));
    }
}

/// Returns the cursor position of the given folder
/// in the absolute visible filetree items list.
/// Defaults to 0 if the folder was not found.
pub fn folder_index(fs: &Filesystem, config: &Config, key: FolderKey) -> usize {
    let mut index = 0;

    if recurse_folder_index(fs, fs.root(), config, key, &mut index) {
        index
    } else {
        0
    }
}

/// Recusively search for the given folder key,
/// counting the number of visible items along the way.
fn recurse_folder_index(
    fs: &Filesystem,
    folder: &Folder,
    config: &Config,
    needle: FolderKey,
    count: &mut usize,
) -> bool {
    let folders = &folder.child_folders;
    for key in folders {
        if *key == needle {
            return true;
        }

        let folder = fs.folder(*key);

        // Skip the folder if it is hidden and not open
        if folder.hidden(config) && !folder.open {
            continue;
        }

        // Count the folder itself
        *count += 1;

        if folder.open {
            if recurse_folder_index(fs, folder, config, needle, count) {
                return true;
            }

            *count += folder.child_files.len();
        }
    }

    false
}
