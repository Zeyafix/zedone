use zedone_core::{
    cards::{CardStore, CreateCard},
    columns::ColumnStore,
    spaces::SpaceStore,
};
pub async fn seed(
    storage: &zedone_storage::Storage,
    admin: std::sync::Arc<zedone_storage::access::PendingAccess>,
) {
    storage
        .transact(move |ctx| {
            let space = format!("{:032x}", 1);
            let column = format!("{:032x}", 2);
            ctx.create_space(&space, "Ежедневные копии", admin.subject())?;
            ctx.create_column(&space, &column, "Мысли")?;
            ctx.create_card(
                &CreateCard {
                    space,
                    column,
                    card: format!("{:032x}", 3),
                    title: "Сохранённый текст".into(),
                    description: "Версия из ежедневной копии.".into(),
                },
                admin.subject(),
                &format!("{:032x}", 4),
            )?;
            Ok(())
        })
        .await
        .unwrap();
}
