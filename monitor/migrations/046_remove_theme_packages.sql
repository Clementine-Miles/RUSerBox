UPDATE desk_preferences SET theme = NULL;
UPDATE desk_preferences SET wallpaper = 'preset:bloom' WHERE wallpaper = 'theme';
DELETE FROM desk_theme_background;
DELETE FROM desk_theme;
